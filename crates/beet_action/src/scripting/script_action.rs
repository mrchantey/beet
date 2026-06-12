use crate::prelude::*;
use beet_core::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Runs the caller's [`Script`] component as a pure `Input -> Output`
/// transformation.
///
/// Requires a [`Script`] sibling (via `#[require]`), so adding a `ScriptAction`
/// is enough to make a scripted entity callable as a behaviour-tree leaf.
///
/// ## Errors
///
/// Errors if the caller has no matching [`Script`] component, or if the
/// script fails to parse, evaluate, or (de)serialize its values.
#[action]
#[derive(Component)]
#[require(Script<Input, Output>)]
pub fn ScriptAction<Input, Output>(
	cx: In<ActionContext<Input>>,
	scripts: Query<&Script<Input, Output>>,
) -> Result<Output>
where
	Input: 'static + Send + Sync + Serialize,
	Output: 'static + Send + Sync + DeserializeOwned,
{
	let entity = cx.id();
	let script = scripts.get(entity).map_err(|err| {
		bevyhow!("ScriptAction caller {entity:?} has no Script: {err}")
	})?;
	script.run(cx.input)
}
