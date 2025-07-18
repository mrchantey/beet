mod dom_binding;
mod load_client_islands;
pub use dom_binding::*;
pub use load_client_islands::*;
mod client_only;
mod event_playback;
use crate::prelude::*;
use bevy::prelude::*;
use client_only::*;
use event_playback::*;


pub fn wasm_template_plugin(app: &mut App) {
	#[cfg(not(test))]
	console_error_panic_hook::set_once();
	if web_sys::window().map(|w| w.document()).flatten().is_none() {
		// no document, probably deno
		#[cfg(not(test))]
		beet_utils::log!(
			"No html document found, skipping wasm template plugin setup"
		);
		return;
	}
	app.add_systems(
		Update,
		(
			load_client_islands.run_if(run_once).before(TemplateSet),
			(
				mount_client_only,
				// the below could be 'parallel' but we're single-threaded anyway
				event_playback.run_if(run_once),
				bind_events,
				bind_text_nodes,
				bind_attribute_values,
			)
				.chain()
				.after(TemplateSet)
				.before(SignalsSet),
			(update_text_nodes, update_attribute_values).after(SignalsSet),
		),
	);
}
