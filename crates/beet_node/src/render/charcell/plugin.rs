use beet_core::prelude::*;




pub struct CharcellPlugin;

impl Plugin for CharcellPlugin {
	fn build(&self, app: &mut App) {
		#[cfg(feature = "tui")]
		app.add_plugins(bevy_ratatui::RatatuiPlugins {
			enable_kitty_protocol: true,
			enable_mouse_capture: true,
			enable_input_forwarding: true,
		});
		app.add_systems(PostUpdate, render_changed);
	}
}
fn render_changed() {
	//TODO
}
