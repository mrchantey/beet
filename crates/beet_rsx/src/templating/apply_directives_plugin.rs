use super::*;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use std::str::FromStr;

#[derive(Default)]
pub struct ApplyDirectivesPlugin;


/// A schedule for completely building templates,
/// this will run before each [`Update`] schedule and can be
/// executed manually after adding unresolved templates to the world.
/// (see beet_server bundle_layer.rs for an example)
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct ApplyDirectives;


pub(crate) fn schedule_order_plugin(app: &mut App) {
	app.insert_schedule_before(Update, ApplyDirectives)
		.insert_schedule_after(ApplyDirectives, PropagateSignals);
}

impl Plugin for ApplyDirectivesPlugin {
	fn build(&self, app: &mut App) {
		bevy::ecs::error::GLOBAL_ERROR_HANDLER
			.set(bevy::ecs::error::panic)
			.ok();
		#[cfg(all(target_arch = "wasm32", not(test)))]
		console_error_panic_hook::set_once();


		app.init_plugin(schedule_order_plugin)
			.add_plugins((SignalsPlugin, NodeTypesPlugin))
			.init_resource::<HtmlConstants>()
			.init_resource::<WorkspaceConfig>()
			.init_resource::<ClientIslandRegistry>()
			.init_resource::<TemplateFlags>()
			.add_systems(
				Startup,
				(
					|| {},
					#[cfg(not(target_arch = "wasm32"))]
					load_all_file_snippets.run_if(TemplateFlags::should_run(
						TemplateFlag::LoadSnippets,
					)),
					#[cfg(target_arch = "wasm32")]
					load_client_islands.run_if(document_exists),
				),
			)
			.add_systems(
				ApplyDirectives,
				// almost all of these systems must be run in this sequence,
				// with one or two exceptions but we're single threaded anyway (faster cold-start)
				(
					apply_rsx_snippets,
					apply_style_id_attributes,
					apply_slots,
					apply_static_lang_snippets,
					apply_requires_dom_idx,
					#[cfg(all(target_arch = "wasm32", not(test)))]
					apply_client_island_dom_idx,
					// in cl
					#[cfg(any(not(target_arch = "wasm32"), test))]
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
					#[cfg(target_arch = "wasm32")]
					(
						mount_client_only,
						event_playback.run_if(run_once),
						bind_events,
						bind_text_nodes,
						bind_attribute_values,
					)
						.chain()
						.run_if(document_exists),
				)
					.chain(),
			);
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
