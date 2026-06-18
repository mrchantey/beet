use beet_core::prelude::*;
use rquickjs::Context;
use rquickjs::Function;
use rquickjs::Runtime;
use rquickjs::Value;
use rquickjs::function::MutFn;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Evaluate a QuickJS `script` as a pure `Input -> Output` function.
///
/// `input` is serialized to JSON and bound to the `input` global; the value of
/// the script's final expression is JSON-stringified and deserialized as the
/// output. JSON is QuickJS's native marshalling currency, so unlike the rhai
/// backend this needs no intermediary `Value` hop — `serde_json` (alloc) is
/// already `no_std`. QuickJS errors are flattened to a message.
pub fn run_quickjs<Input, Output>(script: &str, input: Input) -> Result<Output>
where
	Input: Serialize,
	Output: DeserializeOwned,
{
	let input = serde_json::to_string(&input)
		.map_err(|err| bevyhow!("quickjs: failed to encode input: {err}"))?;

	let runtime = Runtime::new().map_err(|err| bevyhow!("quickjs: {err}"))?;
	let context =
		Context::full(&runtime).map_err(|err| bevyhow!("quickjs: {err}"))?;

	context.with(|ctx| {
		// bind `input` by parsing the JSON encoding into a live value.
		ctx.globals()
			.set("input", ctx.json_parse(input)?)
			.map_err(|err| bevyhow!("quickjs: failed to bind input: {err}"))?;

		let output = ctx
			.eval::<Value, _>(script)
			.map_err(|err| bevyhow!("quickjs: {err}"))?;
		let output = ctx
			.json_stringify(output)
			.map_err(|err| bevyhow!("quickjs: {err}"))?
			.ok_or_else(|| bevyhow!("quickjs: script returned no value"))?
			.to_string()
			.map_err(|err| bevyhow!("quickjs: {err}"))?;

		serde_json::from_str(&output)
			.map_err(|err| bevyhow!("quickjs: failed to decode output: {err}"))
	})
}

/// Which host stream a [`run_quickjs_console`] call targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleStream {
	/// `console.log`/`info`/`debug`.
	Stdout,
	/// `console.warn`/`error`.
	Stderr,
}

/// The `console` shim installed before a [`run_quickjs_console`] script: each call
/// formats its args and forwards them straight to the `__console_write` FFI sink, so
/// output reaches the host the moment the call runs (not buffered until `eval`
/// returns). The IIFE returns `undefined`, leaving no stray value.
const CONSOLE_PRELUDE: &str = r#"
(() => {
	const fmt = (args) => args
		.map((arg) => typeof arg === 'string' ? arg : JSON.stringify(arg))
		.join(' ');
	const write = (stream) => (...args) =>
		globalThis.__console_write(stream, fmt(args));
	globalThis.console = {
		log: write(0), info: write(0), debug: write(0),
		warn: write(1), error: write(1),
	};
})();
"#;

/// Evaluate `script` for its side effects, streaming each `console` call to `sink`
/// the moment it runs.
///
/// Unlike [`run_quickjs`] (a pure `Input -> Output` transform), `console`
/// `log`/`info`/`debug` forward to [`ConsoleStream::Stdout`] and `warn`/`error` to
/// [`ConsoleStream::Stderr`] through a direct FFI binding, not a buffer read back
/// after `eval` runs. So a long-running or async script's output is not held back
/// until `eval` returns (it may never). After the top-level eval the QuickJS job
/// queue is drained, so a script that schedules a microtask (a resolved promise,
/// `queueMicrotask`) runs to completion, its output streaming as each job runs.
/// Tolerates a script that returns no value (a bare `console.log("hi")`, which
/// [`run_quickjs`] rejects). The `input` global is the serialized `input`, as in
/// [`run_quickjs`]. This is the `EvalOnLoad` eval path.
///
/// `sink` is `FnMut` and runs on the single-threaded [`Context::full`], so it needs
/// no `Send`.
pub fn run_quickjs_console<Input, Sink>(
	script: &str,
	input: Input,
	sink: Sink,
) -> Result<()>
where
	Input: Serialize,
	Sink: 'static + FnMut(ConsoleStream, &str),
{
	let input = serde_json::to_string(&input)
		.map_err(|err| bevyhow!("quickjs: failed to encode input: {err}"))?;

	let runtime = Runtime::new().map_err(|err| bevyhow!("quickjs: {err}"))?;
	let context =
		Context::full(&runtime).map_err(|err| bevyhow!("quickjs: {err}"))?;

	context.with(|ctx| -> Result<()> {
		let globals = ctx.globals();
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
			.map_err(|err| bevyhow!("quickjs: console prelude: {err}"))?;
		ctx.eval::<Value, _>(script)
			.map_err(|err| bevyhow!("quickjs: {err}"))?;
		Ok(())
	})?;

	// drain scheduled jobs (microtasks) so a promise-based script completes, its
	// console output streaming as each job runs.
	while runtime
		.execute_pending_job()
		.map_err(|err| bevyhow!("quickjs: job: {err:?}"))?
	{}
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn increments_a_number() {
		AsyncPlugin::world()
			.spawn((
				Script::<i64, i64>::quickjs("input + 1"),
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
				Script::<String, String>::quickjs(r#""hello " + input"#),
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
				Script::<Player, Player>::quickjs("input.score += 10; input"),
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

	#[beet_core::test]
	async fn parse_errors_propagate() {
		AsyncPlugin::world()
			.spawn((
				Script::<i64, i64>::quickjs("this is not valid js ((("),
				ScriptAction::<i64, i64>::default(),
			))
			.call::<i64, i64>(0)
			.await
			.unwrap_err();
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
		run_quickjs_console(script, input, move |stream, msg| {
			let mut out = sink.borrow_mut();
			match stream {
				ConsoleStream::Stdout => out.stdout.push(msg.to_string()),
				ConsoleStream::Stderr => out.stderr.push(msg.to_string()),
			}
		})
		.unwrap();
		// the sink (and its `Rc` clone) is dropped with the context inside
		// `run_quickjs_console`, leaving `output` the sole owner.
		Rc::try_unwrap(output).unwrap().into_inner()
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
			serde_json::json!({ "name": "ada" }),
		);
		output.stdout.xpect_eq(vec!["ada".to_string()]);
		output.stderr.xpect_eq(vec!["oops".to_string()]);
	}

	/// The job queue drains after the top-level eval, so a microtask scheduled by a
	/// resolved promise still runs and its output streams. The old
	/// buffer-after-eval shim missed it.
	#[beet_core::test]
	fn drains_async_microtasks() {
		let output =
			capture(r#"Promise.resolve().then(() => console.log("later"))"#, ());
		output.stdout.xpect_eq(vec!["later".to_string()]);
	}
}
