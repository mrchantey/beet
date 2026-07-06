//! Wires the perceive-act tools into the `beet` binary.
use super::*;
use crate::beet::prelude::*;
use beet_core::prelude::*;

/// Adds the agent-thread runtime + chat UI and the capability-binding glue, and
/// registers the perceive-act tools and their state, so a `examples/perceive_act/*.bsx`
/// scene runs and its `<TakePhoto/>`, `<RespondMultiModalAction/>`, `<SpeakText/>`,
/// `<LogDriveForDuration/>` and `<SetEmotion/>` tags resolve from markup.
pub struct PerceiveActPlugin;

impl Plugin for PerceiveActPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<ThreadPlugin>()
			.init_plugin::<ThreadUiPlugin>()
			.add_plugins(CapabilityBindingPlugin)
			.init_resource::<PhotoStream>()
			.register_type::<TakePhoto>()
			.register_type::<PostPhoto>()
			.register_type::<RespondMultiModalAction>()
			.register_type::<RespondMultiModal>()
			.register_type::<SpeakText>()
			.register_type::<LogDriveForDuration>()
			.register_type::<SetEmotion>()
			.register_type::<DriveForDuration>()
			.register_template::<MockHead>()
			.register_template::<MockBody>();
		// the wgpu render body (v2): the driven fox and its `drive` handler, so
		// `<WgpuBody/>` resolves once the render stack is linked.
		#[cfg(feature = "bevy_default")]
		app.register_type::<DriveFox>()
			.register_template::<WgpuBody>();
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
		app.world_mut().spawn(children![
			RespondMultiModalAction,
			SpeakText,
			LogDriveForDuration,
			SetEmotion
		]);
		app.world_mut().flush();
		app.world_mut()
			.run_system_once(|tools: Query<(), With<ToolDefinition>>| {
				tools.iter().count()
			})
			.unwrap()
			.xpect_eq(4);
	}
}
