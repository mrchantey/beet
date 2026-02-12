//! # CLI Server
//!
//! Demonstrates a single-shot CLI application using [`cli_server`].
//! The app parses process arguments, dispatches them through a
//! [`markdown_interface`], and prints the response to stdout.
//!
//! A [`Card`] with no [`PathPartial`] matches the empty path,
//! serving as the root content when no arguments are provided.
//!
//! ## Running the Example
//!
//! ```sh
//! # show root card
//! cargo run --example cli_server --features stack
//!
//! # show help for all routes
//! cargo run --example cli_server --features stack -- --help
//!
//! # navigate to a card
//! cargo run --example cli_server --features stack -- about
//!
//! # show help scoped to a subcommand
//! cargo run --example cli_server --features stack -- counter --help
//!
//! # not-found shows help for nearest ancestor
//! cargo run --example cli_server --features stack -- counter nonsense
//!
//! # directional navigation
//! cargo run --example cli_server --features stack -- --navigate=first-child
//! cargo run --example cli_server --features stack -- about --navigate=next-sibling
//! cargo run --example cli_server --features stack -- about --navigate=parent
//! ```
use beet::prelude::*;
mod my_stack;

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), StackPlugin))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((cli_server(), my_stack::my_stack()));
		})
		.run()
}
