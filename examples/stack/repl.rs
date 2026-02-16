//! # REPL Server
//!
//! Demonstrates an interactive REPL application using [`repl_server`].
//! The app reads lines from stdin, dispatches each through a
//! [`default_interface`], and prints the response to stdout.
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
//! > --navigate=first-child
//! > about --navigate=next-sibling
//! > exit
//! ```
use beet::prelude::*;
mod petes_beets;

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), StackPlugin))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((repl_server(), petes_beets::stack()));
		})
		.run()
}
