//! Isolated JavaScript evaluation in a wasm host (browser or Deno), streaming the
//! script's console output.
//!
//! The wasm counterpart to the native quickjs/rhai runtimes, backing
//! `Script::run_console` (in `beet_action`) on wasm: it runs a `<script>` body in
//! the wasm host, capturing `console` output and streaming it through a sink the
//! same shape as the native side ([`ConsoleStream`] + `FnMut(stream, &str)`).
//!
//! The script runs with `console` overridden for the eval and restored after, so
//! its output is captured wherever it runs (Deno, the wasm test runner, or a
//! browser). Browser origin/document isolation via a sandboxed iframe is a planned
//! refinement; today the eval shares the host realm, which is moot under Deno (the
//! tested path, no document to isolate from).

use crate::prelude::*;
use wasm_bindgen::prelude::*;

/// Which host stream a console call targets, mirroring the native runtime's
/// `ConsoleStream` so the console path dispatches to the same sink shape on both
/// targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleStream {
	/// `console.log`/`info`/`debug`.
	Stdout,
	/// `console.warn`/`error`.
	Stderr,
}

/// The keys the eval binds on `globalThis` for the console bridge, the script
/// source, and the input value, removed again after the eval so the host realm is
/// left clean.
const BRIDGE_KEY: &str = "__beet_console_write";
const SCRIPT_KEY: &str = "__beet_script";
const INPUT_KEY: &str = "__beet_input";

/// Evaluate `script` in the wasm host, streaming each `console` call to `sink` the
/// moment it runs.
///
/// `console` `log`/`info`/`debug` forward to [`ConsoleStream::Stdout`] and
/// `warn`/`error` to [`ConsoleStream::Stderr`]. `input` is converted to a live JS
/// value ([`value_to_js`]) and bound as the global `input`, the wasm analogue of the
/// native runtime's `input` — beet's own [`Value`], so no JSON round-trip. `console`
/// and `input` are overridden for the eval and restored after. `sink` is captured
/// into a `'static` JS closure, so a capturing test sink shares its buffer through an
/// `Rc`.
pub fn eval_console(
	script: &str,
	input: &Value,
	mut sink: impl 'static + FnMut(ConsoleStream, &str),
) -> Result<()> {
	let global = js_sys::global();
	// the bridge every console call lands on, forwarding to the sink immediately.
	let bridge = Closure::<dyn FnMut(i32, String)>::new(
		move |stream: i32, msg: String| {
			let stream = match stream {
				1 => ConsoleStream::Stderr,
				_ => ConsoleStream::Stdout,
			};
			sink(stream, &msg);
		},
	);
	set_global(&global, BRIDGE_KEY, bridge.as_ref())?;
	set_global(&global, SCRIPT_KEY, &JsValue::from_str(script))?;
	set_global(&global, INPUT_KEY, &value_to_js(input))?;

	// run the script with a forwarding `console` and the parsed `input`, restoring
	// both on the way out. The script and input are read from `globalThis` rather
	// than interpolated, so no escaping is needed; indirect `eval` runs it in the
	// surrounding realm.
	let runner = format!(
		r#"(function() {{
	const savedConsole = globalThis.console;
	const savedInput = globalThis.input;
	const fmt = (args) => args
		.map((arg) => typeof arg === 'string' ? arg : JSON.stringify(arg))
		.join(' ');
	// forward each console call to the bridge with the real console restored for the
	// duration, so a sink that itself logs (eg streaming to the host console) does
	// not re-enter this override and recurse into the bridge closure.
	const write = (stream) => (...args) => {{
		const line = fmt(args);
		globalThis.console = savedConsole;
		try {{ globalThis.{BRIDGE_KEY}(stream, line); }}
		finally {{ globalThis.console = overridden; }}
	}};
	const overridden = {{
		log: write(0), info: write(0), debug: write(0),
		warn: write(1), error: write(1),
	}};
	globalThis.console = overridden;
	globalThis.input = globalThis.{INPUT_KEY};
	try {{ (0, eval)(globalThis.{SCRIPT_KEY}); }}
	finally {{
		globalThis.console = savedConsole;
		globalThis.input = savedInput;
	}}
}})()"#
	);

	let result = js_sys::eval(&runner);
	// drop the bridge and clear the temporary globals before surfacing any error.
	delete_global(&global, BRIDGE_KEY);
	delete_global(&global, SCRIPT_KEY);
	delete_global(&global, INPUT_KEY);
	drop(bridge);
	result.map_err(|err| bevyhow!("script_ext: eval failed: {err:?}"))?;
	Ok(())
}

/// Convert a beet [`Value`] into a live [`JsValue`] for binding as the script
/// `input`, the wasm analogue of the native runtimes' input marshalling.
///
/// Mirrors the shape `JSON.parse` of the native JSON encoding would yield: numbers
/// (incl. bytes) become JS numbers, a [`Value::Bytes`] an array of byte numbers, a
/// [`Value::List`] an array, and a [`Value::Map`] an object with string keys.
fn value_to_js(value: &Value) -> JsValue {
	match value {
		Value::Null => JsValue::NULL,
		Value::Bool(bool) => JsValue::from_bool(*bool),
		Value::Int(int) => JsValue::from_f64(*int as f64),
		Value::Uint(uint) => JsValue::from_f64(*uint as f64),
		Value::Float(float) => JsValue::from_f64(*float),
		Value::Str(str) => JsValue::from_str(str),
		Value::Bytes(bytes) => bytes
			.iter()
			.map(|byte| JsValue::from_f64(*byte as f64))
			.collect::<js_sys::Array>()
			.into(),
		Value::List(list) => list
			.iter()
			.map(value_to_js)
			.collect::<js_sys::Array>()
			.into(),
		Value::Map(map) => {
			let obj = js_sys::Object::new();
			map.iter().for_each(|(key, value)| {
				js_sys::Reflect::set(
					&obj,
					&JsValue::from_str(key.as_str()),
					&value_to_js(value),
				)
				.ok();
			});
			obj.into()
		}
	}
}

/// Set `key` on `target`, mapping a JS error to a [`Result`].
fn set_global(
	target: &js_sys::Object,
	key: &str,
	value: &JsValue,
) -> Result<()> {
	js_sys::Reflect::set(target, &JsValue::from_str(key), value)
		.map(|_| ())
		.map_err(|err| bevyhow!("script_ext: bind `{key}`: {err:?}"))
}

/// Remove `key` from `target`, ignoring a failure (cleanup is best effort).
fn delete_global(target: &js_sys::Object, key: &str) {
	js_sys::Reflect::delete_property(target, &JsValue::from_str(key)).ok();
}

#[cfg(test)]
mod test {
	use super::ConsoleStream;
	use super::eval_console;
	use crate::prelude::*;
	use std::cell::RefCell;
	use std::rc::Rc;

	/// `console.log`/`error` from an evaluated script stream to the sink, split by
	/// stream. Runs under the Deno wasm test runner.
	#[beet_core::test]
	fn streams_console_to_sink() {
		let out = Rc::new(RefCell::new(Vec::<(ConsoleStream, String)>::new()));
		let sink = out.clone();
		eval_console(
			r#"console.log("hello"); console.error("oops")"#,
			&Value::Null,
			move |stream, msg| {
				sink.borrow_mut().push((stream, msg.to_string()))
			},
		)
		.unwrap();
		let out = out.borrow();
		out.len().xpect_eq(2);
		out[0].0.xpect_eq(ConsoleStream::Stdout);
		out[0].1.xpect_eq("hello".to_string());
		out[1].0.xpect_eq(ConsoleStream::Stderr);
		out[1].1.xpect_eq("oops".to_string());
	}

	/// The `input_json` is parsed and bound as the global `input`, so a script reads
	/// it the same way the native runtime exposes `input`.
	#[beet_core::test]
	fn binds_input() {
		let out = Rc::new(RefCell::new(Vec::<String>::new()));
		let sink = out.clone();
		eval_console(
			r#"console.log(input.name)"#,
			&val!({ "name": "ada" }),
			move |_, msg| sink.borrow_mut().push(msg.to_string()),
		)
		.unwrap();
		out.borrow().clone().xpect_eq(vec!["ada".to_string()]);
	}
}
