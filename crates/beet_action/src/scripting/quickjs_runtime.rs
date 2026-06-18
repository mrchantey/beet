use beet_core::prelude::*;
use rquickjs::Array;
use rquickjs::Context;
use rquickjs::Runtime;
use rquickjs::Value;
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

/// Console output captured from a side-effecting [`run_quickjs_console`] eval.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ConsoleOutput {
	/// Lines from `console.log`/`info`/`debug`, in call order.
	pub stdout: Vec<String>,
	/// Lines from `console.warn`/`error`, in call order.
	pub stderr: Vec<String>,
}

/// The `console` shim installed before a [`run_quickjs_console`] script: each
/// call buffers a formatted line into a global array the host reads back, so
/// output reaches the host streams with no Rust callback binding. The IIFE
/// returns `undefined`, leaving no stray value.
const CONSOLE_PRELUDE: &str = r#"
globalThis.__stdout = [];
globalThis.__stderr = [];
(() => {
	const fmt = (args) => args
		.map((arg) => typeof arg === 'string' ? arg : JSON.stringify(arg))
		.join(' ');
	globalThis.console = {
		log:   (...args) => globalThis.__stdout.push(fmt(args)),
		info:  (...args) => globalThis.__stdout.push(fmt(args)),
		debug: (...args) => globalThis.__stdout.push(fmt(args)),
		warn:  (...args) => globalThis.__stderr.push(fmt(args)),
		error: (...args) => globalThis.__stderr.push(fmt(args)),
	};
})();
"#;

/// Evaluate `script` for its side effects, returning its captured console output.
///
/// Unlike [`run_quickjs`] (a pure `Input -> Output` transform), this binds a
/// `console` whose `log`/`info`/`debug` buffer to stdout and `warn`/`error` to
/// stderr, and tolerates a script that returns no value (a statement like
/// `console.log("hi")`, which [`run_quickjs`] rejects). The `input` global is the
/// serialized `input`, as in [`run_quickjs`]. This is the `StartScript` eval path.
pub fn run_quickjs_console<Input>(
	script: &str,
	input: Input,
) -> Result<ConsoleOutput>
where
	Input: Serialize,
{
	let input = serde_json::to_string(&input)
		.map_err(|err| bevyhow!("quickjs: failed to encode input: {err}"))?;

	let runtime = Runtime::new().map_err(|err| bevyhow!("quickjs: {err}"))?;
	let context =
		Context::full(&runtime).map_err(|err| bevyhow!("quickjs: {err}"))?;

	context.with(|ctx| {
		let globals = ctx.globals();
		globals
			.set("input", ctx.json_parse(input)?)
			.map_err(|err| bevyhow!("quickjs: failed to bind input: {err}"))?;

		// install the buffering `console`, then run the script. Both eval to a
		// discarded `Value`, so a no-value statement never errors.
		ctx.eval::<Value, _>(CONSOLE_PRELUDE)
			.map_err(|err| bevyhow!("quickjs: console prelude: {err}"))?;
		ctx.eval::<Value, _>(script)
			.map_err(|err| bevyhow!("quickjs: {err}"))?;

		// drain each buffer array back into the host strings, in call order.
		let drain = |key: &str| -> Result<Vec<String>> {
			let array: Array = globals
				.get(key)
				.map_err(|err| bevyhow!("quickjs: read `{key}`: {err}"))?;
			(0..array.len())
				.map(|index| {
					array.get::<String>(index).map_err(|err| {
						bevyhow!("quickjs: decode `{key}`: {err}")
					})
				})
				.collect()
		};
		Ok(ConsoleOutput {
			stdout: drain("__stdout")?,
			stderr: drain("__stderr")?,
		})
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

	#[beet_core::test]
	fn console_log_captures_stdout() {
		let output =
			run_quickjs_console(r#"console.log("hello world")"#, ()).unwrap();
		output.stdout.xpect_eq(vec!["hello world".to_string()]);
		output.stderr.xpect_empty();
	}

	#[beet_core::test]
	fn console_reads_input_and_splits_streams() {
		let output = run_quickjs_console(
			r#"console.log(input.name); console.error("oops")"#,
			serde_json::json!({ "name": "ada" }),
		)
		.unwrap();
		output.stdout.xpect_eq(vec!["ada".to_string()]);
		output.stderr.xpect_eq(vec!["oops".to_string()]);
	}
}
