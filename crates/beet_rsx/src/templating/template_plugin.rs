use super::*;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::schedule::SystemSet;
use bevy::prelude::*;
use std::str::FromStr;

/// System set for the [`TemplatePlugin`] to spawn templates.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct TemplateSet;

#[derive(Default)]
pub struct TemplatePlugin;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BuildTemplate;


impl Plugin for TemplatePlugin {
	fn build(&self, app: &mut App) {
		bevy::ecs::error::GLOBAL_ERROR_HANDLER
			.set(bevy::ecs::error::panic)
			.ok();
		#[cfg(all(feature = "serde", not(target_arch = "wasm32")))]
		app.add_systems(
			Startup,
			load_all_file_snippets
				.run_if(TemplateFlags::should_run(TemplateFlag::LoadSnippets)),
		);
		app.add_plugins((SignalsPlugin, NodeTypesPlugin))
			.init_resource::<HtmlConstants>()
			.init_resource::<WorkspaceConfig>()
			.init_resource::<ClientIslandRegistry>()
			.init_resource::<TemplateFlags>()
			.add_systems(
				Update,
				// almost all of these systems must be run in this sequence,
				// with one or two exceptions but we're single threaded anyway (faster cold-start)
				(
					apply_static_rsx,
					apply_style_id_attributes,
					apply_slots,
					apply_static_lang_snippets,
					apply_requires_dom_idx,
					#[cfg(target_arch = "wasm32")]
					apply_client_island_dom_idx,
					#[cfg(not(target_arch = "wasm32"))]
					apply_root_dom_idx,
					rearrange_html_document,
					apply_reactive_text_nodes,
					#[cfg(feature = "scene")]
					apply_client_islands,
					insert_hydration_scripts,
					hoist_document_elements,
					insert_event_playback_attribute,
					compress_style_ids,
					render_html_fragments,
				)
					.chain()
					.in_set(TemplateSet),
			);
		#[cfg(target_arch = "wasm32")]
		app.add_plugins(wasm_template_plugin);
	}
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Resource, Reflect)]
#[reflect(Resource)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
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
