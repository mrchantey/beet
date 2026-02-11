//! # REPL Server
//!
//! Demonstrates an interactive REPL application using [`repl_server`].
//! The app reads lines from stdin, dispatches each through a
//! [`markdown_interface`], and prints the response to stdout.
//!
//! ## Running the Example
//!
//! ```sh
//! cargo run --example repl --features stack
//! ```
//!
//! Then type commands interactively:
//! ```text
//! > --help
//! > increment
//! > about
//! > exit
//! ```
use beet::prelude::*;

fn main() -> AppExit {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, LogPlugin::default(), StackPlugin));
	app.world_mut().spawn((
		Card,
		markdown_interface(),
		repl_server(),
		children![increment(FieldRef::new("count")), card("about"),],
	));
	app.run()
}
