use beet_core::prelude::*;
use bevy_ratatui::RatatuiPlugins;




pub struct TuiPlugin2;

impl Plugin for TuiPlugin2 {
	fn build(&self, app: &mut App) {
		app.add_plugins(RatatuiPlugins {
			enable_kitty_protocol: true,
			enable_mouse_capture: true,
			enable_input_forwarding: true,
		})
		.add_systems(PostUpdate, super::render);
	}
}
