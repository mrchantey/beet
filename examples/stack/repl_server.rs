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
//! > about
//! > counter --help
//! > counter increment
//! > exit
//! ```
use beet::prelude::*;
mod my_stack;

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), StackPlugin))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((repl_server(), my_stack::my_stack()));
		})
		.run()
}
