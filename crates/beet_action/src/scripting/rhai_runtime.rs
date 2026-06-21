use beet_core::prelude::*;
use rhai::Dynamic;
use rhai::Engine;
use rhai::Scope;
use serde::Serialize;
use serde::de::DeserializeOwned;
// the console runtime (and its `Rc`-shared sink) is native-only: on wasm the host
// JS realm is the `console`.
#[cfg(not(target_arch = "wasm32"))]
use crate::prelude::ConsoleStream;
#[cfg(not(target_arch = "wasm32"))]
use alloc::rc::Rc;
#[cfg(not(target_arch = "wasm32"))]
use core::cell::RefCell;

/// Evaluate a rhai `script` as a pure `Input -> Output` function.
///
/// `input` is marshalled to a [`Value`] and bound to the `input` variable in
/// scope; the value of the script's final expression is marshalled back through
/// [`Value`] and deserialized as the output. Going through [`Value`] rather than
/// `rhai::serde` keeps the bridge `no_std` (rhai's own serde layer needs std), so
/// the same runtime serves embedded and host targets. rhai errors are not
/// `Send + Sync`, so they are flattened to a message.
pub fn run_rhai<Input, Output>(script: &str, input: Input) -> Result<Output>
where
	Input: Serialize,
	Output: DeserializeOwned,
{
	let input = Value::from_serde(input)?;
	let mut scope = Scope::new();
	scope.push_dynamic("input", value_to_dynamic(&input));

	let output = Engine::new()
		.eval_with_scope::<Dynamic>(&mut scope, script)
		.map_err(|err| bevyhow!("rhai: {err}"))?;

	dynamic_to_value(output).into_serde()
}

/// Evaluate a rhai `script` for its side effects, streaming each console line to
/// `sink` the moment it runs.
///
/// rhai has no `console`, so its print builtins are the console channel:
/// [`print`](https://rhai.rs/book/language/print-debug.html) streams as
/// [`ConsoleStream::Stdout`] and `debug` as [`ConsoleStream::Stderr`], mirroring
/// JS `console.log`/`console.error`. `input` is bound as in [`run_rhai`]; the
/// script's return value is discarded, so a script that returns nothing is fine.
/// The rhai backend of [`Script::run_console`].
///
/// `sink` is `FnMut` but rhai's `on_print`/`on_debug` hooks are `Fn`, so it is
/// shared through an `Rc<RefCell<_>>` both hooks borrow. rhai builds non-`sync`
/// here, so neither the hooks nor the sink need `Send`.
///
/// Native-only: on wasm a `<script>` console runs in the host JS realm (the only
/// `console`), so [`Script::run_console`] takes the host path there regardless of
/// the script's language.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_rhai_console<Input, Sink>(
	script: &str,
	input: Input,
	sink: Sink,
) -> Result<()>
where
	Input: Serialize,
	Sink: 'static + FnMut(ConsoleStream, &str),
{
	let input = Value::from_serde(input)?;
	let mut scope = Scope::new();
	scope.push_dynamic("input", value_to_dynamic(&input));

	let sink = Rc::new(RefCell::new(sink));
	let print_sink = sink.clone();
	let mut engine = Engine::new();
	engine.on_print(move |line| {
		print_sink.borrow_mut()(ConsoleStream::Stdout, line)
	});
	// rhai's `debug` hook also reports the source and position; only the message
	// maps onto the console channel.
	engine.on_debug(move |line, _src, _pos| {
		sink.borrow_mut()(ConsoleStream::Stderr, line)
	});

	engine
		.eval_with_scope::<Dynamic>(&mut scope, script)
		.map(|_| ())
		.map_err(|err| bevyhow!("rhai: {err}"))
}

/// Marshal a [`Value`] into a rhai [`Dynamic`].
fn value_to_dynamic(value: &Value) -> Dynamic {
	match value {
		Value::Null => Dynamic::UNIT,
		Value::Bool(val) => (*val).into(),
		Value::Int(val) => (*val).into(),
		// rhai has no unsigned type, so widen into its signed integer.
		Value::Uint(val) => (*val as i64).into(),
		Value::Float(val) => (*val).into(),
		Value::Str(val) => val.as_str().into(),
		Value::Bytes(val) => Dynamic::from_array(
			val.iter()
				.map(|byte| (*byte as i64).into())
				.collect::<rhai::Array>(),
		),
		Value::Map(map) => Dynamic::from_map(
			map.iter()
				.map(|(key, value)| {
					(key.as_str().into(), value_to_dynamic(value))
				})
				.collect::<rhai::Map>(),
		),
		Value::List(list) => Dynamic::from_array(
			list.iter().map(value_to_dynamic).collect::<rhai::Array>(),
		),
	}
}

/// Marshal a rhai [`Dynamic`] back into a [`Value`].
fn dynamic_to_value(value: Dynamic) -> Value {
	if value.is_unit() {
		Value::Null
	} else if value.is_bool() {
		Value::Bool(value.as_bool().unwrap_or_default())
	} else if value.is_int() {
		Value::Int(value.as_int().unwrap_or_default())
	} else if value.is_float() {
		Value::Float(value.as_float().unwrap_or_default())
	} else if value.is_string() {
		Value::Str(value.into_string().unwrap_or_default().into())
	} else if value.is_array() {
		Value::List(
			value
				.into_array()
				.unwrap_or_default()
				.into_iter()
				.map(dynamic_to_value)
				.collect(),
		)
	} else if value.is_map() {
		Value::Map(Map(value
			.cast::<rhai::Map>()
			.into_iter()
			.map(|(key, value)| (key.as_str().into(), dynamic_to_value(value)))
			.collect()))
	} else {
		Value::Null
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn increments_a_number() {
		AsyncPlugin::world()
			.spawn((
				Script::<i64, i64>::rhai("input + 1"),
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
				Script::<String, String>::rhai(r#""hello " + input"#),
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
				Script::<Player, Player>::rhai("input.score += 10; input"),
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
	async fn multi_statement_script() {
		AsyncPlugin::world()
			.spawn((
				Script::<i64, i64>::rhai("let x = input * 2; x + 3"),
				ScriptAction::<i64, i64>::default(),
			))
			.call::<i64, i64>(10)
			.await
			.unwrap()
			.xpect_eq(23);
	}

	#[beet_core::test]
	async fn parse_errors_propagate() {
		AsyncPlugin::world()
			.spawn((
				Script::<i64, i64>::rhai("this is not valid rhai ((("),
				ScriptAction::<i64, i64>::default(),
			))
			.call::<i64, i64>(0)
			.await
			.unwrap_err();
	}

	/// Collects the streamed console output into buffers for assertions.
	#[cfg(not(target_arch = "wasm32"))]
	#[derive(Debug, Default)]
	struct ConsoleOutput {
		stdout: Vec<String>,
		stderr: Vec<String>,
	}

	/// Run a script with a capturing sink, collecting its streamed output. The sink
	/// must be `'static`, so it shares the buffer through an `Rc` rather than
	/// borrowing the local.
	#[cfg(not(target_arch = "wasm32"))]
	fn capture(script: &str, input: impl serde::Serialize) -> ConsoleOutput {
		use std::cell::RefCell;
		use std::rc::Rc;
		let output = Rc::new(RefCell::new(ConsoleOutput::default()));
		let sink = output.clone();
		super::run_rhai_console(script, input, move |stream, msg| {
			let mut out = sink.borrow_mut();
			match stream {
				ConsoleStream::Stdout => out.stdout.push(msg.to_string()),
				ConsoleStream::Stderr => out.stderr.push(msg.to_string()),
			}
		})
		.unwrap();
		Rc::try_unwrap(output).unwrap().into_inner()
	}

	/// rhai has no `console`, so `print` is the stdout channel and `debug` the
	/// stderr channel.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	fn print_and_debug_split_streams() {
		let output = capture(
			r#"print("hello " + input.name); debug("oops")"#,
			val!({ "name": "ada" }),
		);
		output.stdout.xpect_eq(vec!["hello ada".to_string()]);
		// rhai `debug` quotes a string literal, mirroring its inspect formatting.
		output.stderr.xpect_eq(vec![r#""oops""#.to_string()]);
	}

	/// `run_captured` joins the stdout lines into a newline-terminated body, the
	/// shape served as a response, and tolerates a script that returns no value.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	fn run_captured_collects_stdout() {
		Script::<(), ()>::rhai(r#"print("a"); print("b")"#)
			.run_captured(())
			.unwrap()
			.xpect_eq("a\nb\n".to_string());
	}
}
