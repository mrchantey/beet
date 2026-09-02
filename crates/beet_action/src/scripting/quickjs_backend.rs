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
use crate::prelude::ScriptExposure;
use crate::prelude::ScriptLimits;
use crate::prelude::WorldCall;
use crate::scripting::dynamic::WORLD_SHIM;
use beet_core::prelude::*;
use rquickjs::CatchResultExt;
use rquickjs::Context;
use rquickjs::Ctx;
use rquickjs::Function;
use rquickjs::Runtime;
use rquickjs::Value;
use rquickjs::function::MutFn;
use serde::Serialize;
use serde::de::DeserializeOwned;
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
		value: Value<'js>,
	) -> Result<Value<'js>> {
		let Some(promise) = value.as_promise() else {
			return Ok(value);
		};
		loop {
			if let Some(result) = promise.result::<Value>() {
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

/// Evaluate a QuickJS `script` as a pure `Input -> Output` function, bounded by
/// `limits`.
///
/// `input` is serialized to JSON and bound to the `input` global; the value of
/// the script's final expression is JSON-stringified and deserialized as the
/// output. JSON is QuickJS's native marshalling currency, so this needs no
/// intermediary `Value` hop — `serde_json` (alloc) is already `no_std`.
pub(crate) fn run_quickjs<Input, Output>(
	script: &str,
	input: Input,
	limits: &ScriptLimits,
) -> Result<Output>
where
	Input: Serialize,
	Output: DeserializeOwned,
{
	// the console forwards to the host log rather than the floor: a stray
	// `console.log` in a pure transform must not throw (it runs fine on every
	// other backend), and a script whose printing silently vanishes is the one
	// thing in beet you cannot debug by printing.
	eval_quickjs(script, input, limits, ConsoleStream::log, None)?
		.ok_or_else(|| bevyhow!("quickjs: script returned no value"))?
		.xmap(|output| serde_json::from_str(&output))
		.map_err(|err| bevyhow!("quickjs: failed to decode output: {err}"))
}

/// Evaluate `script` for its side effects under `limits`, streaming each `console`
/// call to `sink` the moment it runs.
///
/// Unlike [`run_quickjs`] (a pure `Input -> Output` transform), `console`
/// `log`/`info`/`debug` forward to [`ConsoleStream::Stdout`] and `warn`/`error` to
/// [`ConsoleStream::Stderr`] through a direct FFI binding, not a buffer read back
/// after `eval` runs. So a long-running or async script's output is not held back
/// until `eval` returns (it may never). Tolerates a script that returns no value
/// (a bare `console.log("hi")`, which [`run_quickjs`] rejects).
///
/// `sink` is `FnMut` and runs on the single-threaded [`Context::full`], so it needs
/// no `Send`.
pub(crate) fn run_quickjs_console<Input, Sink>(
	script: &str,
	input: Input,
	sink: Sink,
	limits: &ScriptLimits,
) -> Result<()>
where
	Input: Serialize,
	Sink: 'static + FnMut(ConsoleStream, &str),
{
	eval_quickjs(script, input, limits, sink, None).map(|_| ())
}

/// Evaluate `script` with the `world` bridge installed, serving every call it
/// makes against `world` as it makes it.
///
/// The world is reached as a plain `&mut World` rather than through the async
/// bridge, because the engine is synchronous: the whole evaluation happens
/// inside one exclusive world access, and the pump loop below alternates
/// between draining the engine's job queue and answering what the script asked
/// for. That is what makes a read see the write before it.
///
/// The presence of a world is what installs the bridge at all: a plain
/// [`run_quickjs`] leaves the script with no `world` global to reach for.
pub(crate) fn run_quickjs_world<Input>(
	script: &str,
	input: Input,
	world: &mut World,
	exposure: &ScriptExposure,
	limits: &ScriptLimits,
) -> Result<Option<JsonValue>>
where
	Input: Serialize,
{
	eval_quickjs(
		script,
		input,
		limits,
		ConsoleStream::log,
		Some(WorldSeat { world, exposure }),
	)?
	.map(|output| {
		serde_json::from_str(&output)
			.map_err(|err| bevyhow!("quickjs: failed to decode output: {err}"))
	})
	.transpose()
}

/// The world a bridged evaluation serves its calls against, absent for a pure
/// run.
struct WorldSeat<'a> {
	world: &'a mut World,
	exposure: &'a ScriptExposure,
}

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

/// The one embedded evaluation path, returning the JSON encoding of the script's
/// completion value (`None` when it produced none).
///
/// Every public entry point is this function with a different sink, a different
/// use of the result, and with or without a world, so they cannot drift in how
/// they treat `console`, a returned promise or the job queue — a drift that
/// would mean a script running on the deno backends and failing on this one.
fn eval_quickjs<Input, Sink>(
	script: &str,
	input: Input,
	limits: &ScriptLimits,
	sink: Sink,
	seat: Option<WorldSeat<'_>>,
) -> Result<Option<String>>
where
	Input: Serialize,
	Sink: 'static + FnMut(ConsoleStream, &str),
{
	let input = serde_json::to_string(&input)
		.map_err(|err| bevyhow!("quickjs: failed to encode input: {err}"))?;

	let bounded = BoundedRuntime::new(limits)?;

	bounded.context.with(|ctx| {
		let globals = ctx.globals();
		// bind `input` by parsing the JSON encoding into a live value.
		globals
			.set("input", ctx.json_parse(input)?)
			.map_err(|err| bevyhow!("quickjs: failed to bind input: {err}"))?;

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

		// install the streaming `console`, then run the script. Both eval to a
		// discarded `Value`, so a no-value statement never errors.
		ctx.eval::<Value, _>(CONSOLE_PRELUDE)
			.catch(&ctx)
			.map_err(|err| bevyhow!("quickjs: console prelude: {err}"))?;

		// the world bridge, installed only for a world-bridged evaluation: the
		// engine-side hooks, then the shared shim. A plain `run` never gets here,
		// so a pure script has no `world` to reach for.
		let Some(seat) = seat else {
			let output = ctx
				.eval::<Value, _>(script)
				.catch(&ctx)
				.map_err(|err| bounded.error(err))?;
			// settle a returned promise, then drain what is left, so an async
			// script completes and its trailing microtasks get to run.
			let output = bounded.settle(&ctx, output)?;
			bounded.drain_jobs(&ctx)?;
			return ctx
				.json_stringify(output)
				.catch(&ctx)
				.map_err(|err| bounded.error(err))?
				.map(|output| output.to_string())
				.transpose()
				.map_err(|err| bounded.error(err));
		};

		// calls queue here rather than crossing a transport: the engine runs on
		// this thread, so `__world_send_json` is a host function that hands one
		// over and returns, leaving the promise it minted pending until the pump
		// below answers it.
		let queue =
			alloc::rc::Rc::new(core::cell::RefCell::new(Vec::<String>::new()));
		let sender = queue.clone();
		let send = Function::new(
			ctx.clone(),
			MutFn::new(move |json: String| sender.borrow_mut().push(json)),
		)
		.map_err(|err| bevyhow!("quickjs: bind world sink: {err}"))?;
		globals
			.set("__world_send_json", send)
			.map_err(|err| bevyhow!("quickjs: bind world send: {err}"))?;
		ctx.eval::<Value, _>(WORLD_PUMP)
			.catch(&ctx)
			.map_err(|err| bevyhow!("quickjs: world pump: {err}"))?;
		ctx.eval::<Value, _>(WORLD_SHIM)
			.catch(&ctx)
			.map_err(|err| bevyhow!("quickjs: world shim: {err}"))?;

		let output = ctx
			.eval::<Value, _>(script)
			.catch(&ctx)
			.map_err(|err| bounded.error(err))?;
		let begin = globals
			.get::<_, Function>("__world_begin")
			.map_err(|err| bevyhow!("quickjs: world pump: {err}"))?;
		begin
			.call::<_, ()>((output,))
			.catch(&ctx)
			.map_err(|err| bounded.error(err))?;

		let reply = globals
			.get::<_, Function>("__world_reply")
			.map_err(|err| bevyhow!("quickjs: world shim: {err}"))?;
		// the pump: drain what the engine has queued, answer everything the
		// script asked for while it drained, and repeat. A call answered here
		// resolves a promise, which queues more work, which may ask again.
		loop {
			bounded.drain_jobs(&ctx)?;
			if bounded.expired() {
				return Err(bounded.error("serving the world bridge"));
			}
			let calls = core::mem::take(&mut *queue.borrow_mut());
			if calls.is_empty() {
				let Some(done) = globals
					.get::<_, Option<String>>("__world_done")
					.map_err(|err| bevyhow!("quickjs: world pump: {err}"))?
				else {
					// nothing pending and nothing asked for: the script is waiting
					// on a promise nothing will ever settle.
					return Err(bevyhow!(
						"quickjs: the returned promise can never settle: the job queue \
ran dry while it was still pending"
					));
				};
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
			for call in calls {
				let answer = match serde_json::from_str::<WorldCall>(&call) {
					Ok(call) => call.execute(seat.world, seat.exposure),
					// the shim is the only writer, so this can only be a bug in
					// the wire types, and it has no id to correlate.
					Err(err) => {
						bevybail!(
							"quickjs: malformed world call `{call}`: {err}"
						)
					}
				};
				let answer = serde_json::to_string(&answer).map_err(|err| {
					bevyhow!("quickjs: failed to encode world reply: {err}")
				})?;
				reply
					.call::<_, ()>((ctx.json_parse(answer)?,))
					.catch(&ctx)
					.map_err(|err| bounded.error(err))?;
			}
		}
	})
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn increments_a_number() {
		AsyncPlugin::world()
			.spawn((
				Script::<i64, i64>::new("input + 1"),
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
				Script::<String, String>::new(r#""hello " + input"#),
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
				Script::<Player, Player>::new("input.score += 10; input"),
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
		Script::<(), i64>::new("Promise.resolve(20).then(value => value * 2)")
			.run(())
			.await
			.unwrap()
			.xpect_eq(40);
	}

	/// A promise that can never settle is an error, not a hang: the job queue
	/// running dry with it still pending says so outright.
	#[beet_core::test]
	async fn an_unsettleable_promise_errors() {
		Script::<(), i64>::new("new Promise(() => {})")
			.run(())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("never settle");
	}

	/// Parity again: every other backend installs a `console`, so a stray
	/// `console.log` in a pure transform must not be a `ReferenceError` here.
	#[beet_core::test]
	async fn a_pure_run_still_has_a_console() {
		Script::<(), i64>::new(r#"console.log("noise"); 7"#)
			.run(())
			.await
			.unwrap()
			.xpect_eq(7);
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
		Script::<(), Vec<String>>::new(
			"[typeof fetch, typeof Deno, typeof process, typeof require]",
		)
		.run(())
		.await
		.unwrap()
		.xpect_eq(vec!["undefined".to_string(); 4]);
	}

	/// A runaway loop is cut off at the wall-clock deadline by the interrupt
	/// handler, and the host is unharmed: the next script runs normally.
	#[beet_core::test]
	async fn infinite_loop_stops_at_the_deadline() {
		Script::<(), ()>::new("while (true) {}")
			.with_limits(ScriptLimits {
				timeout: Duration::from_millis(200),
				..default()
			})
			.run(())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("timed out");
		// the host survived the containment
		Script::<i64, i64>::new("input + 1")
			.run(1)
			.await
			.unwrap()
			.xpect_eq(2);
	}

	/// An endlessly re-queueing microtask chain is bounded too: each individual
	/// job returns, so only the drain loop's own deadline check stops it.
	#[beet_core::test]
	async fn runaway_microtasks_stop_at_the_deadline() {
		Script::<(), ()>::new(
			"const loop = () => Promise.resolve().then(loop); loop()",
		)
		.with_limits(ScriptLimits {
			timeout: Duration::from_millis(200),
			..default()
		})
		.run_console((), |_, _| {})
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("timed out");
	}

	/// An allocation bomb is contained by the memory cap, not the clock.
	#[beet_core::test]
	async fn allocation_bomb_hits_the_memory_cap() {
		Script::<(), ()>::new(
			"const held = []; while (true) held.push(new Array(100000).fill(0))",
		)
		.with_limits(ScriptLimits {
			memory: 8 * 1024 * 1024,
			..default()
		})
		.run(())
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("out of memory");
	}

	/// Runaway recursion exhausts the capped interpreter stack as a `RangeError`
	/// the script itself can catch, rather than overflowing the host stack.
	#[beet_core::test]
	async fn deep_recursion_is_a_catchable_range_error() {
		Script::<(), bool>::new(
			r#"(() => {
				try {
					(function recurse() { return 1 + recurse() })();
					return false;
				} catch (err) { return err instanceof RangeError }
			})()"#,
		)
		.run(())
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
	fn capture(script: &str, input: impl serde::Serialize) -> ConsoleOutput {
		use std::cell::RefCell;
		use std::rc::Rc;
		let output = Rc::new(RefCell::new(ConsoleOutput::default()));
		let sink = output.clone();
		super::run_quickjs_console(
			script,
			input,
			move |stream, msg| {
				let mut out = sink.borrow_mut();
				match stream {
					ConsoleStream::Stdout => out.stdout.push(msg.to_string()),
					ConsoleStream::Stderr => out.stderr.push(msg.to_string()),
				}
			},
			&ScriptLimits::default(),
		)
		.unwrap();
		// the sink (and its `Rc` clone) is dropped with the context inside
		// `run_quickjs_console`, leaving `output` the sole owner.
		Rc::try_unwrap(output).unwrap().into_inner()
	}

	/// Values `JSON.stringify` cannot render (undefined, BigInt) still print as
	/// something, rather than as an empty string or a thrown `TypeError`.
	#[beet_core::test]
	fn console_formats_unstringifiable_values() {
		let output = capture(r#"console.log(undefined, 1n, "x")"#, ());
		output.stdout.xpect_eq(vec!["undefined 1 x".to_string()]);
	}

	#[beet_core::test]
	fn console_log_streams_stdout() {
		let output = capture(r#"console.log("hello world")"#, ());
		output.stdout.xpect_eq(vec!["hello world".to_string()]);
		output.stderr.xpect_empty();
	}

	#[beet_core::test]
	fn console_reads_input_and_splits_streams() {
		let output = capture(
			r#"console.log(input.name); console.error("oops")"#,
			value!({ "name": "ada" }),
		);
		output.stdout.xpect_eq(vec!["ada".to_string()]);
		output.stderr.xpect_eq(vec!["oops".to_string()]);
	}

	/// The job queue drains after the top-level eval, so a microtask scheduled by a
	/// resolved promise still runs and its output streams. The old
	/// buffer-after-eval shim missed it.
	#[beet_core::test]
	fn drains_async_microtasks() {
		let output = capture(
			r#"Promise.resolve().then(() => console.log("later"))"#,
			(),
		);
		output.stdout.xpect_eq(vec!["later".to_string()]);
	}
}
