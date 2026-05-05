use beet_core::prelude::*;

pub struct CharcellPlugin;

impl Plugin for CharcellPlugin {
	#[allow(unused_variables)]
	fn build(&self, app: &mut App) {
		cfg_if! {
			if #[cfg(feature = "tui")] {
				app.add_plugins(bevy_ratatui::RatatuiPlugins {
					enable_kitty_protocol: true,
					enable_mouse_capture: true,
					enable_input_forwarding: true,
				});
			} else if #[cfg(feature = "crossterm")] {
				use crate::prelude::*;
				app
					.add_systems(PostUpdate, (
						render_crossterm::<std::io::Stdout>,
						render_crossterm::<Vec<u8>>,
					));
			}
		}
	}
}
