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
/// Async, like [`Script::run`] itself: the embedded engine resolves immediately,
/// while every other backend is a child process or host isolate reached over a
/// message channel.
///
/// ## Errors
///
/// Errors if the caller has no matching [`Script`] component, or if the
/// script fails to parse, evaluate, or (de)serialize its values.
#[action]
#[derive(Component)]
#[require(Script<Input, Output>)]
pub async fn ScriptAction<Input, Output>(
	cx: ActionContext<Input>,
) -> Result<Output>
where
	Input: 'static + Send + Sync + Serialize,
	Output: 'static + Send + Sync + DeserializeOwned,
{
	let entity = cx.id();
	// the script is cloned out of the world rather than borrowed: the eval is
	// awaited, and the world moves on in the meantime.
	let script = cx
		.world()
		.with_state::<Query<&Script<Input, Output>>, _>(move |scripts| {
			scripts.get(entity).ok().cloned()
		})
		.await
		.ok_or_else(|| {
			bevyhow!("ScriptAction caller {entity:?} has no Script")
		})?;
	script.run(cx.take()).await
}
