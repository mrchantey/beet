use beet_core::prelude::*;
use rquickjs::Context;
use rquickjs::Runtime;
// beet's `Value` is the marshalling currency; rquickjs's is the live engine
// value, aliased to keep the two apart.
use rquickjs::Value as JsValue;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Evaluate a QuickJS `script` as a pure `Input -> Output` function.
///
/// `input` is marshalled to a [`Value`], JSON-encoded and bound to the `input`
/// global; the value of the script's final expression is JSON-stringified, read
/// back into a [`Value`] and deserialized as the output. Routing through
/// [`Value`] (then JSON across the engine boundary) matches the rhai backend, so
/// every engine shares one marshalling currency. QuickJS errors are flattened to
/// a message.
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
			.eval::<JsValue, _>(script)
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
}
