//! Wires the perceive-act tools into the `beet` binary.
use super::*;
use crate::beet::prelude::*;
use beet_core::prelude::*;

/// Adds the agent-thread runtime + chat UI and the capability-binding glue, and
/// registers the perceive-act tools and their state, so a `examples/perceive_act/*.bsx`
/// scene runs and its `<TakePhoto/>`, `<RespondMultiModalAction/>`, `<SpeakText/>`,
/// `<LogDriveForDuration/>` and `<ShowImage/>` tags resolve from markup, and the camera
/// turn rotates scenes via `{SceneRotation}` on the router.
pub struct PerceiveActPlugin;

impl Plugin for PerceiveActPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<ThreadPlugin>()
			.init_plugin::<ThreadUiPlugin>()
			.init_plugin::<PerceiveActCorePlugin>()
			.add_plugins(CapabilityBindingPlugin)
			.init_resource::<PhotoStream>()
			.init_resource::<RandomSource>()
			.register_type::<TakePhoto>()
			.register_type::<PostPhoto>()
			.register_type::<RespondMultiModalAction>()
			.register_type::<RespondMultiModal>()
			.register_type::<SpeakText>()
			.register_type::<LogDriveForDuration>()
			.register_type::<DriveForDuration>()
			// scene rotation: config + state components, and the image-options sync.
			.register_type::<SceneRotation>()
			.register_type::<SceneCatalog>()
			.register_type::<ActiveScene>()
			.register_type::<SceneImage>()
			.register_type::<SceneOrder>()
			.register_type::<ScenePrompt>()
			.add_systems(Update, sync_image_options)
			.register_template::<MockHead>()
			.register_template::<MockBody>()
			.register_template::<RobotStreamer>();
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
			ShowImage
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
