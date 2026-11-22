use crate::prelude::*;
use beet_core::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Runs the caller as a [`Script`], an `Input -> Output` transformation under
/// the caller's [`ScriptConfig`].
///
/// The program is the caller's own [`Value`] and its descendants'
/// ([`ElementTextQuery::script_source`]), the one shape a script takes, read
/// per call so an edit to the source runs next time. The config is read from
/// the same entity and defaults when absent, which is the whole
/// world-capability story for a typed script: an `ExchangeScript` route gets
/// it for free.
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
/// Errors if the caller is gone, or if the script fails to parse, evaluate,
/// or (de)serialize its values.
#[action(local)]
#[derive(Component)]
pub async fn ScriptAction<Input, Output>(
	cx: ActionContext<Input>,
) -> Result<Output>
where
	Input: 'static + Send + Sync + Serialize,
	Output: 'static + Send + Sync + DeserializeOwned,
{
	let entity = cx.id();
	let world = cx.world().clone();
	// the source and its grant are read out of the world rather than
	// borrowed: the eval is awaited, and the world moves on in the meantime.
	let (source, config) = world
		.with_state::<(ElementTextQuery, Query<Option<&ScriptConfig>>), _>(
			move |(text, configs)| {
				configs
					.get(entity)
					.map(|config| (text.script_source(entity), config.cloned()))
			},
		)
		.await
		.map_err(|_| bevyhow!("the script {entity:?} was despawned"))?;
	Script::<Input, Output>::new(source)
		.run(cx.take(), world.clone(), &config.unwrap_or_default())
		.await
}
