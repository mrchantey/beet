//! Wires the perceive-act tools into the `beet` binary.
use super::*;
use crate::beet::prelude::*;
use beet_core::prelude::*;

/// Adds the agent-thread runtime + chat UI and registers the perceive-act tools and
/// their state, so a `examples/perceive_act/*.bsx` scene runs and its `<TakePhoto/>`,
/// `<Remark/>`, `<SetHeading/>` and `<SetEmotion/>` tags resolve from markup.
pub struct PerceiveActPlugin;

impl Plugin for PerceiveActPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<ThreadPlugin>()
			.init_plugin::<ThreadUiPlugin>()
			.init_resource::<PhotoStream>()
			.register_type::<TakePhoto>()
			.register_type::<Remark>()
			.register_type::<SetHeading>()
			.register_type::<SetEmotion>()
			.register_type::<Heading>()
			.register_type::<Emotion>();
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// Each tool's `route` + reflected input + doc description must resolve to a
	/// `ToolDefinition` the agent can offer the model. Network-free: it never streams.
	#[beet_core::test]
	fn tools_resolve_to_definitions() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_plugin::<ThreadPlugin>();
		app.world_mut()
			.spawn(children![TakePhoto, Remark, SetHeading, SetEmotion]);
		app.world_mut().flush();
		app.world_mut()
			.run_system_once(|tools: Query<(), With<ToolDefinition>>| {
				tools.iter().count()
			})
			.unwrap()
			.xpect_eq(4);
	}
}
