use crate::prelude::*;
use bevy::ecs::schedule::SystemSet;
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct SpawnStep;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ApplyTransformsStep;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct RenderStep;

#[derive(Default)]
pub struct TemplatePlugin;


impl Plugin for TemplatePlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			(
				ApplyTransformsStep.after(SpawnStep),
				RenderStep.after(ApplyTransformsStep),
				ApplySlotsStep.in_set(ApplyTransformsStep),
			),
		)
		.add_plugins((apply_slots_plugin, render_html_plugin))
		.set_runner(SignalAppRunner::runner);
		#[cfg(target_arch = "wasm32")]
		app.add_plugins(wasm_template_plugin);
	}
}
