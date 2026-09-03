//! Shared fixtures for the [`Script`] tests.
use crate::prelude::*;
pub(crate) use crate::scripting::dynamic::test_support::test_world;
use beet_core::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Run `script` against a throwaway async world under the default
/// [`ScriptConfig`].
pub(crate) async fn run_script<Input, Output>(
	script: &str,
	input: Input,
) -> Result<Output>
where
	Input: 'static + Send + Sync + Serialize,
	Output: 'static + Send + Sync + DeserializeOwned,
{
	run_script_with(script, input, ScriptConfig::default()).await
}

/// [`run_script`], under a config other than the default.
///
/// Drives its own world to completion, since every evaluation is handed to a
/// local task and resolves at a sync point.
pub(crate) async fn run_script_with<Input, Output>(
	script: &str,
	input: Input,
	config: ScriptConfig,
) -> Result<Output>
where
	Input: 'static + Send + Sync + Serialize,
	Output: 'static + Send + Sync + DeserializeOwned,
{
	let script = Script::<Input, Output>::new(script);
	AsyncPlugin::world()
		.run_async_local_then(move |world| async move {
			script.run(input, world, &config).await
		})
		.await
}

/// Spawn a scripted leaf on `world` and call it as a behaviour tree would, the
/// shape every world-bridge test takes.
pub(crate) async fn run_leaf(
	world: &mut World,
	script: &str,
) -> Result<Outcome> {
	run_leaf_with(world, script, ScriptConfig::default()).await
}

/// [`run_leaf`], under a config other than the default.
pub(crate) async fn run_leaf_with(
	world: &mut World,
	script: &str,
	config: ScriptConfig,
) -> Result<Outcome> {
	world
		.spawn((
			Script::<Value, Value>::new(script),
			OutcomeScript::<Value, Value>::default(),
			config,
		))
		.call::<(), Outcome>(())
		.await
}

/// The value a script left on `entity`.
pub(crate) fn read_value(
	world: &mut World,
	entity: Entity,
	ident: &str,
) -> Value {
	WorldRead::get(world, entity, ident, &ScriptConfig::default())
		.unwrap()
		.unwrap()
}
