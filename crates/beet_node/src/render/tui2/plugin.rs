use beet_core::prelude::*;




pub struct TuiPlugin2;

impl Plugin for TuiPlugin2 {
	fn build(&self, app: &mut App) {
		#[cfg(feature = "tui")]
		app.add_plugins(bevy_ratatui::RatatuiPlugins {
			enable_kitty_protocol: true,
			enable_mouse_capture: true,
			enable_input_forwarding: true,
		});
		app.add_systems(PostUpdate, super::render_changed);
	}
}
