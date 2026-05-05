use std::io::Stdout;

use beet_core::prelude::*;

use crate::render::CharcellRenderer;
use crate::render::CrosstermBackend;




pub struct CharcellPlugin;

impl Plugin for CharcellPlugin {
	fn build(&self, app: &mut App) {
		cfg_if! {
			if #[cfg(feature = "tui")]{
				app.add_plugins(bevy_ratatui::RatatuiPlugins {
					enable_kitty_protocol: true,
					enable_mouse_capture: true,
					enable_input_forwarding: true,
				});
			}else{
				app
					.insert_resource(CrosstermBackend::<Stdout>::default())
					.add_systems(PostUpdate, render_crossterm);
			}
		}
	}
}

#[allow(unused)]
fn render_crossterm(
	mut backend: Res<CrosstermBackend<Stdout>>,
	query: Populated<&CharcellRenderer, Changed<CharcellRenderer>>,
) {
	println!("Rendering..")
}
