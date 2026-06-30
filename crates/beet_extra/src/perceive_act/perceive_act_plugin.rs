//! Wires the perceive-act tools into the `beet` binary.
use super::*;
use crate::beet::prelude::*;
use beet_core::prelude::*;

/// Adds the agent-thread runtime + chat UI and registers the perceive-act tools and
/// their state, so a `examples/perceive_act/*.bsx` scene runs and its
/// `<PerceiveActToolset/>` (and the individual `<TakePhoto/>` etc tags) resolve.
pub struct PerceiveActPlugin;

impl Plugin for PerceiveActPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<ThreadPlugin>()
			.init_plugin::<ThreadUiPlugin>()
			.init_resource::<PhotoStream>()
			.init_resource::<CurrentEmotion>()
			.init_resource::<LastHeading>()
			.register_type::<TakePhoto>()
			.register_type::<Remark>()
			.register_type::<Drive>()
			.register_type::<SetEmotion>()
			.register_template::<PerceiveActToolset>();
	}
}

/// `<PerceiveActToolset/>` — equips the enclosing agent with the four perceive-act
/// tools, nested as children so the thread query discovers them.
#[template]
pub fn PerceiveActToolset() -> impl Bundle {
	children![TakePhoto, Remark, Drive, SetEmotion]
}

#[cfg(test)]
mod test {
	use super::*;

	/// Each tool's `route` + reflected input + doc description must resolve to a
	/// `ToolDefinition` the agent can offer the model. Network-free: it never streams.
	#[beet_core::test]
	fn toolset_resolves_four_tools() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_plugin::<ThreadPlugin>();
		app.world_mut()
			.spawn(children![TakePhoto, Remark, Drive, SetEmotion]);
		app.world_mut().flush();
		app.world_mut()
			.run_system_once(|tools: Query<(), With<ToolDefinition>>| {
				tools.iter().count()
			})
			.unwrap()
			.xpect_eq(4);
	}
}
