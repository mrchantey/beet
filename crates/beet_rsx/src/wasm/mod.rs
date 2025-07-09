mod dom_binding;
pub use dom_binding::*;
mod client_only;
mod event_playback;
use event_playback::*;


use crate::prelude::*;
use beet_core::node::ClientOnlyDirective;
use bevy::prelude::*;
use client_only::*;


pub fn wasm_template_plugin(app: &mut App) {
	console_error_panic_hook::set_once();
	// if web_sys::window().map(|w| w.document()).flatten().is_none() {
	// 	// no document, probably deno
	// 	return;
	// }

	// client-only stuff
	app.world_mut()
		.register_component_hooks::<ClientOnlyDirective>()
		.on_add(on_add_client_only);
	// dom-binding stuff
	app.add_systems(
		Update,
		(
			(
				mount_html,
				(
					bind_events,
					event_playback.run_if(run_once),
					bind_text_nodes,
					bind_attribute_values,
				)
			)
				.chain()
				.after(TemplateSet)
				.before(SignalsSet),
			(update_text_nodes, update_attribute_values).after(SignalsSet),
		),
	);
}
