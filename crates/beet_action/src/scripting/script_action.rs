use crate::prelude::*;
use beet_core::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Runs the caller's [`Script`] as an `Input -> Output` transformation, under
/// the caller's [`ScriptConfig`].
///
/// Requires a [`Script`] sibling (via `#[require]`), so adding a `ScriptAction`
/// is enough to make a scripted entity callable as a behaviour-tree leaf. The
/// config is read from the same entity and defaults when absent, which is the
/// whole world-capability story for a typed script: an `ExchangeScript` route
/// gets it for free.
///
/// Async, like [`Script::run`] itself: the embedded engine resolves immediately,
/// while every other backend is a child process or host isolate reached over a
/// message channel. `local`, so the evaluation shares the world thread: every
/// `world` call the script makes is a bridge round trip, and a task on the
/// shared pool belongs to a worker thread under `bevy_multithreaded`, which a
/// sync point can only wake and hope for, one call per frame.
///
/// ## Errors
///
/// Errors if the caller has no matching [`Script`] component, or if the
/// script fails to parse, evaluate, or (de)serialize its values.
#[action(local)]
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
	let world = cx.world().clone();
	// the script and its grant are cloned out of the world rather than
	// borrowed: the eval is awaited, and the world moves on in the meantime.
	let (script, config) = world
		.with_state::<Query<(&Script<Input, Output>, Option<&ScriptConfig>)>, _>(
			move |scripts| {
				scripts
					.get(entity)
					.map(|(script, config)| (script.clone(), config.cloned()))
					.ok()
			},
		)
		.await
		.ok_or_else(|| {
			bevyhow!("ScriptAction caller {entity:?} has no Script")
		})?;
	script
		.run(cx.take(), world.clone(), &config.unwrap_or_default())
		.await
}
