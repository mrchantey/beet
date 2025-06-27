mod apply_lang_partials;
mod apply_static_nodes;
mod apply_style_id_attributes;
mod html_fragment;
mod apply_on_spawn_template;
pub use apply_static_nodes::*;
pub use apply_style_id_attributes::*;
use beet_bevy::prelude::WorldMutExt;
pub use html_fragment::*;
pub use apply_on_spawn_template::*;
mod html_document;
pub use html_document::*;
mod apply_dom_idx;
mod text_node_parent;
pub use apply_dom_idx::*;
pub use apply_lang_partials::*;
pub use text_node_parent::*;
mod template;
pub use template::*;
mod apply_slots;
use crate::prelude::*;
#[allow(unused)]
pub use apply_slots::*;
use beet_common::prelude::*;
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
		bevy::ecs::error::GLOBAL_ERROR_HANDLER
			.set(bevy::ecs::error::panic)
			.ok();


		app.add_plugins((signals_plugin, NodeTypesPlugin));
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
						spawn_templates,
						(apply_lang_partials, apply_style_id_attributes),
						apply_slots,
						apply_text_node_parents,
						(
							(apply_root_dom_idx, apply_child_dom_idx),
							(
								rearrange_html_document,
								insert_hydration_scripts,
								hoist_document_elements,
							)
								.chain(),
						),
						insert_event_playback_attribute,
						// debug,
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

#[allow(unused)]
fn debug(world: &mut World) {
	for (entity, dom_idx) in world
		.query_once::<(Entity, &DomIdx)>()
		.into_iter()
		.map(|(e, t)| (e, t.clone()))
		.collect::<Vec<_>>()
		.into_iter()
	{
		for component in world.inspect_entity(entity).unwrap() {
			beet_utils::log!(
				"Entity: {:?}, DomIdx: {}, Component: {:?}",
				entity,
				dom_idx,
				component.name()
			);
		}
	}
}
