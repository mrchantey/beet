use super::*;
use crate::prelude::*;
use beet_common::node::HtmlConstants;
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
		app.init_resource::<HtmlConstants>()
			.configure_sets(
				Update,
				(
					ApplyTransformsStep.after(SpawnStep),
					RenderStep.after(ApplyTransformsStep),
				),
			)
			.add_systems(
				Update,
				(
					(
						apply_slots,
						(apply_tree_idx, apply_text_node_parents),
						add_tree_idx_attributes,
					)
						.chain()
						.in_set(ApplyTransformsStep),
					render_html.in_set(RenderStep),
				),
			)
			.set_runner(SignalAppRunner::runner);
		#[cfg(target_arch = "wasm32")]
		app.add_plugins(wasm_template_plugin);
	}
}
