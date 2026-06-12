use beet_core::prelude::*;
use rhai::Dynamic;
use rhai::Engine;
use rhai::Scope;
use serde::Serialize;
use serde::de::DeserializeOwned;

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
			val.iter().map(|byte| (*byte as i64).into()).collect::<rhai::Array>(),
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
}
