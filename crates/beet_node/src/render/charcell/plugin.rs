#[cfg(feature = "crossterm")]
use crate::render::CharcellRenderer;
#[cfg(feature = "crossterm")]
use crate::render::CrosstermBackend;
use beet_core::prelude::*;
#[cfg(feature = "crossterm")]
use std::io::Stdout;

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
				app
					.insert_resource(CrosstermBackend::<Stdout>::default())
					.add_systems(PostUpdate, render_crossterm);
			}
		}
	}
}

#[cfg(feature = "crossterm")]
fn render_crossterm(
	mut backend: Res<CrosstermBackend<Stdout>>,
	query: Populated<&CharcellRenderer, Changed<CharcellRenderer>>,
) {
	println!("Rendering..")
}
