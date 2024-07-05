#![allow(unused)]
use bevy::a11y::AccessibilityPlugin;
use bevy::app::PanicHandlerPlugin;
use bevy::audio::AudioPlugin;
use bevy::core_pipeline::CorePipelinePlugin;
use bevy::diagnostic::DiagnosticsPlugin;
use bevy::gizmos::GizmoPlugin;
use bevy::gltf::GltfPlugin;
use bevy::input::InputPlugin;
use bevy::log::LogPlugin;
use bevy::pbr::PbrPlugin;
use bevy::prelude::*;
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use bevy::render::RenderPlugin;
use bevy::scene::ScenePlugin;
use bevy::sprite::SpritePlugin;
use bevy::state::app::StatesPlugin;
use bevy::text::TextPlugin;
use bevy::time::TimePlugin;
use bevy::ui::UiPlugin;
use bevy::winit::WinitPlugin;

pub struct MostDefaultPlugins;


impl Plugin for MostDefaultPlugins {
	fn build(&self, app: &mut App) {
		app.add_plugins(
			DefaultPlugins
				.build()
				// defaults
				.disable::<PanicHandlerPlugin>()
				.disable::<LogPlugin>()
				// .disable::<TaskPoolPlugin>()
				// .disable::<TypeRegistrationPlugin>()
				// .disable::<FrameCountPlugin>()
				// .disable::<TimePlugin>()
				// .disable::<TransformPlugin>()
				// .disable::<HierarchyPlugin>()
				.disable::<DiagnosticsPlugin>()
				.disable::<InputPlugin>()
				.disable::<WindowPlugin>()
				// .disable::<AccessibilityPlugin>()
				// .disable::<AssetPlugin>()
				// .disable::<ScenePlugin>()
				.disable::<WinitPlugin>()
				// .disable::<RenderPlugin>()
				// .disable::<ImagePlugin>()
				// .disable::<PipelinedRenderingPlugin>()
				// .disable::<CorePipelinePlugin>()
				// .disable::<SpritePlugin>()
				// .disable::<TextPlugin>()
				// .disable::<UiPlugin>()
				// .disable::<PbrPlugin>()
				// .disable::<GltfPlugin>()
				// .disable::<AudioPlugin>()
				.disable::<GilrsPlugin>()
				// .disable::<AnimationPlugin>()
				// .disable::<GizmoPlugin>()
				// .disable::<StatesPlugin>(),
			//
			// .disable::<DevToolsPlugin>()
			// .disable::<CiTestingPlugin>()
			// .disable::<IgnoreAmbiguitiesPlugin>()
			//
		);
		// .register_type::<ImageScaleMode>();
	}
}
