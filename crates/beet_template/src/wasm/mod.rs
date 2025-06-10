mod dom_reactive;
pub use dom_reactive::*;
mod client_only;
use bevy::prelude::*;
pub use client_only::*;
pub fn wasm_template_plugin(app: &mut App) {
	app.add_plugins((client_only_plugin, dom_reactive_plugin));
}
