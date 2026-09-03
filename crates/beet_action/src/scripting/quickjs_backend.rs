//! The embedded QuickJS engine, the primary [`Script`] backend on every target.
//!
//! One body of engine code serves native and wasm. Native links pristine
//! crates.io `rquickjs`; wasm links a fork adding `wasm32-unknown-unknown`,
//! declared under a renamed key because cargo forbids per-target sources for one
//! dependency name, and aliased straight back here so nothing below has to know.
//!
//! The wasm engine has zero ambient authority by construction: it is the C
//! engine compiled to wasm inside beet's own module, with no host bindings at
//! all. Its one import is a clock, which beet satisfies in-module (see
//! [`__rquickjs_host_now_us`]), so the final artifact imports nothing custom.

#[cfg(target_arch = "wasm32")]
use rquickjs_wasm as rquickjs;

use crate::prelude::ConsoleStream;
use crate::prelude::ScriptConfig;
use crate::prelude::ScriptLimits;
use crate::prelude::WorldBridge;
use crate::prelude::WorldCall;
use crate::prelude::WorldReply;
use crate::scripting::dynamic::WORLD_SHIM;
use alloc::rc::Rc;
use beet_core::prelude::*;
use core::cell::RefCell;
use rquickjs::CatchResultExt;
use rquickjs::Context;
use rquickjs::Ctx;
use rquickjs::Function;
use rquickjs::Runtime;
use rquickjs::Value as JsValue;
use rquickjs::function::MutFn;
use serde_json::Value as JsonValue;

/// The engine's single host import on wasm: microseconds since the unix epoch.
///
/// The wasm build of quickjs has no OS beneath it, so its `Date` implementation
/// reaches out through this one extern, which the fork's shim declares and beet
/// defines here. Defining it *in-module* is what leaves the final artifact with
/// no imports of its own to satisfy.
///
/// The engine's timezone is fixed at UTC (there is no tz database to consult),
/// so this is a wall clock and nothing more.
#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn __rquickjs_host_now_us() -> f64 {
	js_sys::Date::now() * 1000.0
}

/// A QuickJS runtime bounded by a [`ScriptLimits`].
///
/// Memory and stack are caps the engine enforces itself; time is a wall-clock
/// deadline the interrupt handler polls, which is the only one the engine cannot
/// express, so it is carried here to name a timeout in the error rather than
/// surfacing QuickJS's opaque "interrupted".
struct BoundedRuntime {
	/// Held only to own the runtime for as long as its context: the job queue is
	/// driven through `Ctx`, which is reentrant-safe inside `Context::with`,
	/// unlike `Runtime::execute_pending_job`.
	_runtime: Runtime,
	context: Context,
	deadline: Instant,
}

impl BoundedRuntime {
	/// Build a runtime and its context under `limits`.
	fn new(limits: &ScriptLimits) -> Result<Self> {
		let runtime =
			Runtime::new().map_err(|err| bevyhow!("quickjs: {err}"))?;
		// saturating rather than truncating: a `u64` ceiling above `usize::MAX`
		// means "no practical cap", not a wrapped-around tiny one.
		runtime
			.set_memory_limit(limits.memory.try_into().unwrap_or(usize::MAX));
		// always set explicitly, never left to the engine default: see
		// `ScriptLimits::stack`.
		runtime.set_max_stack_size(limits.stack as usize);
		let deadline = Instant::now() + limits.timeout;
		runtime.set_interrupt_handler(Some(Box::new(move || {
			Instant::now() >= deadline
		})));
		let context = Context::full(&runtime)
			.map_err(|err| bevyhow!("quickjs: {err}"))?;
		Self {
			_runtime: runtime,
			context,
			deadline,
		}
		.xok()
	}

	/// Whether the wall-clock budget is spent, ie an error from the engine is the
	/// interrupt handler firing rather than a fault in the script.
	fn expired(&self) -> bool { Instant::now() >= self.deadline }

	/// Flatten an engine error into a message, naming a spent deadline as the
	/// timeout it is.
	fn error(&self, err: impl core::fmt::Display) -> BevyError {
		if self.expired() {
			bevyhow!("quickjs: script timed out")
		} else {
			bevyhow!("quickjs: {err}")
		}
	}

	/// Drive the job queue until `value` settles, if it is a promise at all.
	///
	/// The embedded counterpart of the runner's `await`: a script whose last
	/// expression is a promise yields the value it resolves to, not the promise
	/// object, which is what the deno backends do and what a script author
	/// expects.
	fn settle<'js>(
		&self,
		ctx: &Ctx<'js>,
		value: JsValue<'js>,
	) -> Result<JsValue<'js>> {
		let Some(promise) = value.as_promise() else {
			return Ok(value);
		};
		loop {
			if let Some(result) = promise.result::<JsValue>() {
				return result.catch(ctx).map_err(|err| self.error(err));
			}
			if self.expired() {
				return Err(self.error("awaiting the returned promise"));
			}
			if !ctx.execute_pending_job() {
				return Err(bevyhow!(
					"quickjs: the returned promise can never settle: the job queue \
ran dry while it was still pending"
				));
			}
		}
	}

	/// One pump step: drain the job queue, then take whatever the script asked
	/// for while it drained, or the completion it recorded instead.
	///
	/// One re-entry into the context and no more, so nothing engine-shaped
	/// survives the return and the caller is free to await.
	///
	/// # Errors
	/// Errors when the deadline passes, or when the script is waiting on a
	/// promise nothing will ever settle.
	fn pump(
		&self,
		queue: &alloc::rc::Rc<core::cell::RefCell<Vec<String>>>,
	) -> Result<Pumped> {
		self.context.with(|ctx| {
			self.drain_jobs(&ctx)?;
			if self.expired() {
				return Err(self.error("serving the world bridge"));
			}
			let calls = core::mem::take(&mut *queue.borrow_mut());
			if !calls.is_empty() {
				return Pumped::Calls(calls).xok();
			}
			ctx.globals()
				.get::<_, Option<String>>("__world_done")
				.map_err(|err| bevyhow!("quickjs: world pump: {err}"))?
				.map(Pumped::Done)
				// nothing pending and nothing asked for: the script is waiting
				// on a promise nothing will ever settle.
				.ok_or_else(|| {
					bevyhow!(
						"quickjs: the returned promise can never settle: the job \
queue ran dry while it was still pending"
					)
				})
		})
	}

	/// Hand one reply to the shim, settling the promise its id names.
	fn answer(&self, reply: &WorldReply) -> Result<()> {
		let reply = serde_json::to_string(reply).map_err(|err| {
			bevyhow!("quickjs: failed to encode world reply: {err}")
		})?;
		self.context.with(|ctx| {
			ctx.globals()
				.get::<_, Function>("__world_reply")
				.map_err(|err| bevyhow!("quickjs: world shim: {err}"))?
				.call::<_, ()>((ctx.json_parse(reply)?,))
				.catch(&ctx)
				.map_err(|err| self.error(err))
		})
	}

	/// Drain the scheduled microtask queue so a promise-based script runs to
	/// completion, stopping if the deadline passes (a script that endlessly
	/// re-queues would otherwise spin here forever, each individual job finishing
	/// too quickly for the interrupt handler to see).
	fn drain_jobs(&self, ctx: &Ctx) -> Result<()> {
		while ctx.execute_pending_job() {
			if self.expired() {
				return Err(self.error("job queue"));
			}
		}
		Ok(())
	}
}

/// The `console` shim installed before every embedded eval: each call formats its
/// args and forwards them straight to the `__console_write` FFI sink, so output
/// reaches the host the moment the call runs (not buffered until `eval` returns).
/// The IIFE returns `undefined`, leaving no stray value.
///
/// Deliberately mirrors the shim in `protocol::JS_RUNNER`, which does the same job
/// over a different transport. The formatting must stay identical between them, so
/// a script's console output does not change shape with the backend.
const CONSOLE_PRELUDE: &str = r#"
(() => {
	const show = (arg) => {
		if (typeof arg === 'string') return arg;
		// `JSON.stringify` yields undefined for undefined/functions/symbols and
		// throws on BigInt, so fall back to the string form rather than an
		// empty one.
		try {
			const json = JSON.stringify(arg);
			return json === undefined ? String(arg) : json;
		} catch { return String(arg); }
	};
	const write = (stream) => (...args) =>
		globalThis.__console_write(stream, args.map(show).join(' '));
	globalThis.console = {
		log: write(0), info: write(0), debug: write(0),
		warn: write(1), error: write(1),
	};
})();
"#;

/// The rust half of the bridge the embedded engine cannot get from a host
/// realm: the `__world_send` encoder and the `__world_begin` completion
/// recorder the pump loop reads.
const WORLD_PUMP: &str = include_str!("quickjs_pump.js");

/// How the script's own promise settled, as [`WORLD_PUMP`] recorded it.
#[derive(serde::Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Completion {
	/// It resolved, with the value the script produced (absent when it produced
	/// none).
	Ok {
		#[serde(default)]
		value: Option<JsonValue>,
	},
	/// It rejected, with the flattened failure.
	Err { message: String },
}

/// What one pump step found: either the script is still asking for things, or
/// it has settled.
enum Pumped {
	/// Calls the script made while the job queue drained, in the order it made
	/// them.
	Calls(Vec<String>),
	/// The script's own promise settled, as [`WORLD_PUMP`] recorded it.
	Done(String),
}

/// The one embedded evaluation path: evaluate `source` under `config`, serving
/// every world call it makes through `bridge` as it makes it.
///
/// `source` is an async function body wrapped by [`Script::async_body`], so the
/// evaluation always yields a promise; `input` is bound to the `input` global,
/// and the value the script returns comes back as JSON, absent when it returned
/// none. What globals it sees at all is `config`'s to say: a withheld `console`
/// or `world` is simply not installed, so reaching for it is an ordinary
/// catchable `ReferenceError`.
///
/// The engine is synchronous and its `Context` is `!Send`, so this is a local
/// future: it never holds a `Ctx` across an await. A bridged run pumps between
/// awaits — one re-entry to drain the job queue and take what the script asked
/// for, then each call served with nothing held — which is what lets an
/// operation be asynchronous, and what makes a read see the write before it.
///
/// [`Script::async_body`]: crate::prelude::Script
pub(crate) async fn run_quickjs<Sink>(
	source: &str,
	input: &JsonValue,
	config: &ScriptConfig,
	bridge: Option<&WorldBridge>,
	sink: Sink,
) -> Result<Option<JsonValue>>
where
	Sink: 'static + FnMut(ConsoleStream, &str),
{
	let input = encode_input(input)?;
	let bounded = BoundedRuntime::new(&config.limits)?;
	// a bridged script's calls queue here rather than crossing a transport: the
	// engine runs on this thread, so `__world_send_json` is a host function that
	// hands one over and returns, leaving the promise it minted pending until
	// the pump below answers it.
	let queue = Rc::new(RefCell::new(Vec::<String>::new()));

	// one context entry installs everything and evaluates. An unbridged run
	// settles its promise here and yields the value; a bridged one hands the
	// promise to the pump and yields `None`, since the pump is what records how
	// it settled.
	let settled = bounded.context.with(|ctx| -> Result<Option<String>> {
		install_prelude(&ctx, &input, config, sink)?;
		if bridge.is_some() {
			install_bridge(&ctx, &queue)?;
		}
		let output = ctx
			.eval::<JsValue, _>(source)
			.catch(&ctx)
			.map_err(|err| bounded.error(err))?;
		if bridge.is_some() {
			return ctx
				.globals()
				.get::<_, Function>("__world_begin")
				.map_err(|err| bevyhow!("quickjs: world pump: {err}"))?
				.call::<_, ()>((output,))
				.catch(&ctx)
				.map_err(|err| bounded.error(err))
				.map(|_| None);
		}
		// settle the returned promise, then drain what is left, so trailing
		// microtasks get to run.
		let output = bounded.settle(&ctx, output)?;
		bounded.drain_jobs(&ctx)?;
		ctx.json_stringify(output)
			.catch(&ctx)
			.map_err(|err| bounded.error(err))?
			.map(|output| output.to_string())
			.transpose()
			.map_err(|err| bounded.error(err))
	})?;

	match bridge {
		Some(bridge) => drive_bridge(&bounded, &queue, bridge).await,
		None => Ok(settled),
	}?
	.map(|output| {
		serde_json::from_str(&output)
			.map_err(|err| bevyhow!("quickjs: failed to decode output: {err}"))
	})
	.transpose()
}

/// Bind the host half of the world bridge: the sink the shim sends calls
/// through, then the pump and the shim that use it.
fn install_bridge(
	ctx: &Ctx<'_>,
	queue: &Rc<RefCell<Vec<String>>>,
) -> Result<()> {
	let sender = queue.clone();
	let send = Function::new(
		ctx.clone(),
		MutFn::new(move |json: String| sender.borrow_mut().push(json)),
	)
	.map_err(|err| bevyhow!("quickjs: bind world sink: {err}"))?;
	ctx.globals()
		.set("__world_send_json", send)
		.map_err(|err| bevyhow!("quickjs: bind world send: {err}"))?;
	ctx.eval::<JsValue, _>(WORLD_PUMP)
		.catch(ctx)
		.map_err(|err| bevyhow!("quickjs: world pump: {err}"))?;
	ctx.eval::<JsValue, _>(WORLD_SHIM)
		.catch(ctx)
		.map_err(|err| bevyhow!("quickjs: world shim: {err}"))
		.map(|_| ())
}

/// Serve the script's world calls until its own promise settles, and answer
/// with the JSON it settled with (absent when it produced none).
///
/// A call answered here resolves a promise, which queues more work, which may
/// ask again. The engine is only entered between awaits, never across one.
async fn drive_bridge(
	bounded: &BoundedRuntime,
	queue: &Rc<RefCell<Vec<String>>>,
	bridge: &WorldBridge,
) -> Result<Option<String>> {
	loop {
		match bounded.pump(queue)? {
			Pumped::Done(done) => {
				return match serde_json::from_str::<Completion>(&done)
					.map_err(|err| bevyhow!("quickjs: world pump: {err}"))?
				{
					Completion::Ok { value } => {
						value.map(|value| value.to_string()).xok()
					}
					Completion::Err { message } => {
						Err(bevyhow!("quickjs: {message}"))
					}
				};
			}
			Pumped::Calls(calls) => {
				for call in calls {
					let call = serde_json::from_str::<WorldCall>(&call)
						// the shim is the only writer, so this can only be a bug
						// in the wire types, and it has no id to correlate.
						.map_err(|err| {
							bevyhow!(
								"quickjs: malformed world call `{call}`: {err}"
							)
						})?;
					let reply = bridge.serve(call).await;
					bounded.answer(&reply)?;
				}
			}
		}
	}
}

/// `input` as the JSON encoding the engine parses it from.
fn encode_input(input: &JsonValue) -> Result<String> {
	serde_json::to_string(input)
		.map_err(|err| bevyhow!("quickjs: failed to encode input: {err}"))
}

/// Bind the `input` global, then install the streaming `console` if the config
/// grants one.
///
/// The engine has no ambient `console` of its own, so withholding it is simply
/// not installing it: the script reaches for a global that is not there.
fn install_prelude<Sink>(
	ctx: &Ctx<'_>,
	input: &str,
	config: &ScriptConfig,
	sink: Sink,
) -> Result<()>
where
	Sink: 'static + FnMut(ConsoleStream, &str),
{
	let globals = ctx.globals();
	// bind `input` by parsing the JSON encoding into a live value.
	globals
		.set("input", ctx.json_parse(input)?)
		.map_err(|err| bevyhow!("quickjs: failed to bind input: {err}"))?;
	if !config.console {
		return Ok(());
	}

	// the single FFI sink the `console` prelude forwards every call to. `MutFn`
	// wraps the `FnMut` for QuickJS's reentrant calls; the closure writes through
	// immediately, so output streams as the script runs.
	let mut sink = sink;
	let write = Function::new(
		ctx.clone(),
		MutFn::new(move |stream: i32, msg: String| {
			let stream = match stream {
				1 => ConsoleStream::Stderr,
				_ => ConsoleStream::Stdout,
			};
			sink(stream, &msg);
		}),
	)
	.map_err(|err| bevyhow!("quickjs: bind console sink: {err}"))?;
	globals
		.set("__console_write", write)
		.map_err(|err| bevyhow!("quickjs: bind console: {err}"))?;

	// install the streaming `console`. It evals to a discarded `Value`, so a
	// no-value statement never errors.
	ctx.eval::<JsValue, _>(CONSOLE_PRELUDE)
		.catch(ctx)
		.map_err(|err| bevyhow!("quickjs: console prelude: {err}"))
		.map(|_| ())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::scripting::test_support::*;
	use beet_core::prelude::*;
	use serde_json::Value as JsonValue;

	#[beet_core::test]
	async fn increments_a_number() {
		AsyncPlugin::world()
			.spawn((
				Script::<i64, i64>::new("return input + 1"),
				ScriptAction::<i64, i64>::default(),
			))
			.call::<i64, i64>(41)
			.await
			.unwrap()
			.xpect_eq(42);
	}

	#[beet_core::test]
	async fn concatenates_strings() {
		AsyncPlugin::world()
			.spawn((
				Script::<String, String>::new(r#"return "hello " + input"#),
				ScriptAction::<String, String>::default(),
			))
			.call::<String, String>("world".to_string())
			.await
			.unwrap()
			.xpect_eq("hello world".to_string());
	}

	#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
	struct Player {
		name: String,
		score: i64,
	}

	#[beet_core::test]
	async fn mutates_a_struct_field() {
		AsyncPlugin::world()
			.spawn((
				Script::<Player, Player>::new(
					"input.score += 10; return input",
				),
				ScriptAction::<Player, Player>::default(),
			))
			.call::<Player, Player>(Player {
				name: "ada".to_string(),
				score: 5,
			})
			.await
			.unwrap()
			.xpect_eq(Player {
				name: "ada".to_string(),
				score: 15,
			});
	}

	/// Parity with the out-of-process backends, which run the same source through
	/// a runner that awaits a returned promise. Without this the embedded engine
	/// would JSON-stringify the `Promise` object itself, so a script that works on
	/// the dev-speed backend would fail on the primary one.
	#[beet_core::test]
	async fn awaits_an_async_script() {
		run_script::<(), i64>(
			"return Promise.resolve(20).then(value => value * 2)",
			(),
		)
		.await
		.unwrap()
		.xpect_eq(40);
	}

	/// A promise that can never settle is an error, not a hang: the job queue
	/// running dry with it still pending says so outright.
	#[beet_core::test]
	async fn an_unsettleable_promise_errors() {
		run_script::<(), i64>("return new Promise(() => {})", ())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("never settle");
	}

	/// A script answers with `return`, so a typed output that never arrives is an
	/// error naming the one way to answer, not an opaque serde type mismatch.
	#[beet_core::test]
	async fn a_valueless_script_names_the_return() {
		run_script::<(), String>(r#"console.log("noise")"#, ())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("a script answers with `return`");
	}

	/// The corollary: an output that accepts null tolerates the same script, so
	/// a leaf run for its effects needs no ceremony.
	#[beet_core::test]
	async fn a_valueless_script_suits_a_valueless_output() {
		run_script::<(), ()>(r#"console.log("noise")"#, ())
			.await
			.unwrap();
	}

	/// Parity again: every other backend installs a `console`, so a stray
	/// `console.log` in a pure transform must not be a `ReferenceError` here.
	#[beet_core::test]
	async fn a_pure_run_still_has_a_console() {
		run_script::<(), i64>(r#"console.log("noise"); return 7"#, ())
			.await
			.unwrap()
			.xpect_eq(7);
	}

	/// A withheld global is simply absent, so reaching for it throws the
	/// ordinary error a script can catch.
	#[beet_core::test]
	async fn a_withheld_console_is_a_reference_error() {
		run_script_with::<(), i64>(
			r#"try { console.log("noise"); return 1 } catch (err) { return err instanceof ReferenceError ? 2 : 3 }"#,
			(),
			ScriptConfig::default().without_console(),
		)
		.await
		.unwrap()
		.xpect_eq(2);
	}

	#[beet_core::test]
	async fn parse_errors_propagate() {
		AsyncPlugin::world()
			.spawn((
				Script::<i64, i64>::new("this is not valid js ((("),
				ScriptAction::<i64, i64>::default(),
			))
			.call::<i64, i64>(0)
			.await
			.unwrap_err();
	}

	/// A script has no ambient authority: the host's own globals are simply not
	/// there to reach for.
	#[beet_core::test]
	async fn has_no_host_globals() {
		run_script::<(), Vec<String>>(
			"return [typeof fetch, typeof Deno, typeof process, typeof require]",
			(),
		)
		.await
		.unwrap()
		.xpect_eq(vec!["undefined".to_string(); 4]);
	}

	/// A runaway loop is cut off at the wall-clock deadline by the interrupt
	/// handler, and the host is unharmed: the next script runs normally.
	#[beet_core::test]
	async fn infinite_loop_stops_at_the_deadline() {
		run_script_with::<(), ()>(
			"while (true) {}",
			(),
			ScriptConfig::default().with_limits(ScriptLimits {
				timeout: Duration::from_millis(200),
				..default()
			}),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("timed out");
		// the host survived the containment
		run_script::<i64, i64>("return input + 1", 1)
			.await
			.unwrap()
			.xpect_eq(2);
	}

	/// An endlessly re-queueing microtask chain is bounded too: each individual
	/// job returns, so only the drain loop's own deadline check stops it.
	#[beet_core::test]
	async fn runaway_microtasks_stop_at_the_deadline() {
		run_script_with::<(), ()>(
			"const loop = () => Promise.resolve().then(loop); loop()",
			(),
			ScriptConfig::default().with_limits(ScriptLimits {
				timeout: Duration::from_millis(200),
				..default()
			}),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("timed out");
	}

	/// An allocation bomb is contained by the memory cap, not the clock.
	#[beet_core::test]
	async fn allocation_bomb_hits_the_memory_cap() {
		run_script_with::<(), ()>(
			"const held = []; while (true) held.push(new Array(100000).fill(0))",
			(),
			ScriptConfig::default().with_limits(ScriptLimits {
				memory: 8 * 1024 * 1024,
				..default()
			}),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("out of memory");
	}

	/// Runaway recursion exhausts the capped interpreter stack as a `RangeError`
	/// the script itself can catch, rather than overflowing the host stack.
	#[beet_core::test]
	async fn deep_recursion_is_a_catchable_range_error() {
		run_script::<(), bool>(
			r#"
			try {
				(function recurse() { return 1 + recurse() })();
				return false;
			} catch (err) { return err instanceof RangeError }
			"#,
			(),
		)
		.await
		.unwrap()
		.xpect_true();
	}

	/// Collects the streamed console output into buffers for assertions.
	#[derive(Debug, Default)]
	struct ConsoleOutput {
		stdout: Vec<String>,
		stderr: Vec<String>,
	}

	/// Run a script with a capturing sink, collecting its streamed output. The sink
	/// must be `'static`, so it shares the buffer through an `Rc` rather than
	/// borrowing the local.
	///
	/// Straight to the backend, since the point is the split the host-facing
	/// [`Script::run_captured`] flattens into one body.
	async fn capture(script: &str, input: JsonValue) -> ConsoleOutput {
		use std::cell::RefCell;
		use std::rc::Rc;
		let output = Rc::new(RefCell::new(ConsoleOutput::default()));
		let sink = output.clone();
		super::run_quickjs(
			&Script::<(), ()>::async_body(script),
			&input,
			&ScriptConfig::default(),
			None,
			move |stream, msg| {
				let mut out = sink.borrow_mut();
				match stream {
					ConsoleStream::Stdout => out.stdout.push(msg.to_string()),
					ConsoleStream::Stderr => out.stderr.push(msg.to_string()),
				}
			},
		)
		.await
		.unwrap();
		// the sink (and its `Rc` clone) is dropped with the context inside
		// `run_quickjs`, leaving `output` the sole owner.
		Rc::try_unwrap(output).unwrap().into_inner()
	}

	/// Values `JSON.stringify` cannot render (undefined, BigInt) still print as
	/// something, rather than as an empty string or a thrown `TypeError`.
	#[beet_core::test]
	async fn console_formats_unstringifiable_values() {
		capture(r#"console.log(undefined, 1n, "x")"#, JsonValue::Null)
			.await
			.stdout
			.xpect_eq(vec!["undefined 1 x".to_string()]);
	}

	#[beet_core::test]
	async fn console_log_streams_stdout() {
		let output =
			capture(r#"console.log("hello world")"#, JsonValue::Null).await;
		output.stdout.xpect_eq(vec!["hello world".to_string()]);
		output.stderr.xpect_empty();
	}

	#[beet_core::test]
	async fn console_reads_input_and_splits_streams() {
		let output = capture(
			r#"console.log(input.name); console.error("oops")"#,
			serde_json::json!({ "name": "ada" }),
		)
		.await;
		output.stdout.xpect_eq(vec!["ada".to_string()]);
		output.stderr.xpect_eq(vec!["oops".to_string()]);
	}

	/// The job queue drains after the top-level eval, so a microtask scheduled by a
	/// resolved promise still runs and its output streams. The old
	/// buffer-after-eval shim missed it.
	#[beet_core::test]
	async fn drains_async_microtasks() {
		capture(
			r#"Promise.resolve().then(() => console.log("later"))"#,
			JsonValue::Null,
		)
		.await
		.stdout
		.xpect_eq(vec!["later".to_string()]);
	}
}
