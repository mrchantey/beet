use std::str::FromStr;

use super::*;
use crate::prelude::*;
use beet_bevy::prelude::WorldMutExt;
use beet_common::prelude::*;
use bevy::ecs::schedule::SystemSet;
use bevy::prelude::*;

/// System set for the [`TemplatePlugin`] to spawn templates.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct TemplateSet;

#[derive(Default)]
pub struct TemplatePlugin;
impl Plugin for TemplatePlugin {
	fn build(&self, app: &mut App) {
		bevy::ecs::error::GLOBAL_ERROR_HANDLER
			.set(bevy::ecs::error::panic)
			.ok();
		#[cfg(feature = "serde")]
		app.add_systems(
			Startup,
			load_all_file_snippets
				.run_if(TemplateFlags::should_run(TemplateFlag::LoadSnippets)),
		);
		if !app.is_plugin_added::<TemplateConfig>() {
			app.add_plugins(TemplateConfig::default());
		}
		app.add_plugins((SignalsPlugin, NodeTypesPlugin))
			.init_resource::<TemplateFlags>()
			.add_systems(
				Update,
				(
					spawn_templates,
					apply_style_id_attributes,
					apply_slots,
					apply_lang_snippets,
					apply_text_node_parents,
					(
						#[cfg(target_arch = "wasm32")]
						apply_client_island_dom_idx,
						#[cfg(not(target_arch = "wasm32"))]
						apply_root_dom_idx,
						(
							rearrange_html_document,
							insert_hydration_scripts,
							hoist_document_elements,
						)
							.chain(),
					),
					insert_event_playback_attribute,
					render_html_fragments,
					// debug,
				)
					.chain()
					.in_set(TemplateSet),
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


#[derive(Debug, Default, Clone, PartialEq, Eq, Resource)]
pub enum TemplateFlags {
	/// Run with all flags enabled.
	#[default]
	All,
	/// Run with no flags enabled.
	None,
	/// Only run with the specified flags.
	Only(Vec<TemplateFlag>),
}

impl TemplateFlags {
	pub fn only(flag: TemplateFlag) -> Self { Self::Only(vec![flag]) }
	pub fn contains(&self, flag: TemplateFlag) -> bool {
		match self {
			Self::All => true,
			Self::None => false,
			Self::Only(flags) => flags.contains(&flag),
		}
	}

	/// A predicate system for run_if conditions
	pub fn should_run(flag: TemplateFlag) -> impl Fn(Res<Self>) -> bool {
		move |flags| flags.contains(flag)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateFlag {
	/// Load snippets from the file system.
	LoadSnippets,
}


impl std::fmt::Display for TemplateFlag {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			TemplateFlag::LoadSnippets => write!(f, "snippets"),
		}
	}
}

impl FromStr for TemplateFlag {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"load-snippets" => Ok(TemplateFlag::LoadSnippets),
			_ => Err(format!("Unknown flag: {}", s)),
		}
	}
}
