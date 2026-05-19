use beet_core::prelude::*;
use rhai::Dynamic;
use rhai::Engine;
use rhai::Scope;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Evaluate a rhai `script` as a pure `Input -> Output` function.
///
/// `input` is serialized and bound to the `input` variable in scope; the
/// value of the script's final expression is deserialized as the output.
/// rhai errors are not `Send + Sync`, so they are flattened to a message.
pub fn run_rhai<Input, Output>(script: &str, input: Input) -> Result<Output>
where
	Input: Serialize,
	Output: DeserializeOwned,
{
	let mut scope = Scope::new();
	let input = rhai::serde::to_dynamic(input)
		.map_err(|err| bevyhow!("rhai: failed to encode input: {err}"))?;
	scope.push_dynamic("input", input);

	let output = Engine::new()
		.eval_with_scope::<Dynamic>(&mut scope, script)
		.map_err(|err| bevyhow!("rhai: {err}"))?;

	rhai::serde::from_dynamic(&output)
		.map_err(|err| bevyhow!("rhai: failed to decode output: {err}"))
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn increments_a_number() {
		AsyncPlugin::world()
			.spawn(Script::<i64, i64>::rhai("input + 1"))
			.call::<i64, i64>(41)
			.await
			.unwrap()
			.xpect_eq(42);
	}

	#[beet_core::test]
	async fn concatenates_strings() {
		AsyncPlugin::world()
			.spawn(Script::<String, String>::rhai(r#""hello " + input"#))
			.call::<String, String>("world".to_string())
			.await
			.unwrap()
			.xpect_eq("hello world".to_string());
	}

	#[beet_core::test]
	async fn multi_statement_script() {
		AsyncPlugin::world()
			.spawn(Script::<i64, i64>::rhai("let x = input * 2; x + 3"))
			.call::<i64, i64>(10)
			.await
			.unwrap()
			.xpect_eq(23);
	}

	#[beet_core::test]
	async fn parse_errors_propagate() {
		AsyncPlugin::world()
			.spawn(Script::<i64, i64>::rhai("this is not valid rhai ((("))
			.call::<i64, i64>(0)
			.await
			.unwrap_err();
	}
}
