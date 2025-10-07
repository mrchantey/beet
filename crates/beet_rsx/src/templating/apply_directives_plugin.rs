use super::*;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;

#[derive(Default)]
pub struct ApplyDirectivesPlugin;



/// A schedule for completely building templates,
/// this will run before each [`Update`] schedule and can be
/// executed manually after adding unresolved templates to the world.
/// (see beet_net bundle_layer.rs for an example)
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct ApplyDirectives;


pub(crate) fn schedule_order_plugin(app: &mut App) {
	app.insert_schedule_before(Update, ApplyDirectives)
		.insert_schedule_after(ApplyDirectives, PropagateSignals);
}

impl Plugin for ApplyDirectivesPlugin {
	fn build(&self, app: &mut App) {
		app.try_set_error_handler(bevy::ecs::error::panic);
		#[cfg(target_arch = "wasm32")]
		{
			#[cfg(not(test))]
			console_error_panic_hook::set_once();
			#[cfg(target_arch = "wasm32")]
			app.add_systems(
				Startup,
				load_client_islands.run_if(document_exists),
			);
		}
		app.init_plugin(schedule_order_plugin)
			.add_plugins((SignalsPlugin, NodeTypesPlugin))
			.init_resource::<HtmlConstants>()
			.init_resource::<WorkspaceConfig>()
			.init_resource::<ClientIslandRegistry>()
			.add_systems(
				ApplyDirectives,
				(
					apply_slots,
					apply_lang_snippet_hashes,
					apply_style_id,
					deduplicate_lang_nodes,
					apply_requires_dom_idx,
					#[cfg(all(target_arch = "wasm32", not(test)))]
					apply_client_island_dom_idx,
					// #[cfg(any(not(target_arch = "wasm32"), test))]
					apply_root_dom_idx,
					rearrange_html_document,
					apply_reactive_text_nodes,
					#[cfg(feature = "scene")]
					apply_client_islands,
					insert_hydration_scripts,
					hoist_document_elements,
					insert_event_playback_attribute,
					#[cfg(target_arch = "wasm32")]
					(
						mount_client_only,
						event_playback.run_if(run_once),
						bind_dom_idx_text_nodes,
						bind_dom_idx_elements,
						bind_dom_idx_attributes,
						// bind_events,
					)
						.chain()
						.run_if(document_exists),
				)
					.chain(),
			);
	}
}
