mod apply_style_id_attributes;
mod html_fragment;
mod lang_partial;
mod node_portal;
pub use apply_style_id_attributes::*;
pub use html_fragment::*;
pub use node_portal::*;
mod html_document;
pub use html_document::*;
mod text_node_parent;
mod tree_idx;
pub use lang_partial::*;
pub use text_node_parent::*;
pub use tree_idx::*;
mod template;
pub use template::*;
mod apply_slots;
use crate::prelude::*;
#[allow(unused)]
pub use apply_slots::*;
use beet_common::node::HtmlConstants;
use bevy::ecs::schedule::SystemSet;
use bevy::prelude::*;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct SpawnStep;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ApplyTransformsStep;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct RenderStep;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct MountStep;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct BindStep;
#[derive(Default)]
pub struct TemplatePlugin;
impl Plugin for TemplatePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(signals_plugin);
		app.init_resource::<HtmlConstants>()
			.register_type::<LangPartial>()
			.configure_sets(
				Update,
				(
					ApplyTransformsStep.after(SpawnStep),
					RenderStep.after(ApplyTransformsStep),
					MountStep.after(RenderStep),
					BindStep.after(MountStep),
				),
			)
			.add_systems(
				Update,
				(
					(
						(resolve_lang_partials, apply_style_id_attributes),
						apply_slots,
						apply_text_node_parents,
						(
							apply_tree_idx,
							(
								rearrange_html_document,
								insert_hydration_scripts,
								hoist_document_elements,
							)
								.chain(),
						),
						insert_event_playback_attribute,
					)
						.chain()
						.in_set(ApplyTransformsStep),
					(render_html_fragments).in_set(RenderStep),
				),
			);
		#[cfg(target_arch = "wasm32")]
		app.add_plugins(wasm_template_plugin);
	}
}
