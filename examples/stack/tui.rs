//! # TUI Server
//!
//! Demonstrates an interactive terminal UI application using [`tui_server`]
//! and [`TuiPlugin`]. The app renders a card tree sidebar, a main content
//! panel, and a command palette for dispatching requests.
//!
//! ## Running the Example
//!
//! ```sh
//! cargo run --example tui --features tui
//! ```
//!
//! ## Controls
//!
//! - `Tab` to cycle focus between panels
//! - Type commands in the command palette and press `Enter`
//! - `t` to toggle the card tree sidebar
//! - `q` or `Esc` to quit (when not in command palette)
//! - `Ctrl-C` to quit from anywhere
//!
//! ## Example Commands
//!
//! ```text
//! --help
//! about
//! counter --help
//! counter increment
//! ```
use beet::prelude::*;
mod demo_stack;

fn main() -> AppExit {
	App::new()
		.add_plugins((
			MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
				Duration::from_secs_f32(1. / 60.),
			)),
			LogPlugin::default(),
			StackPlugin,
			TuiPlugin,
		))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((tui_server(), demo_stack::stack()));
		})
		.run()
}
