//! # CLI Server
//!
//! Demonstrates a single-shot CLI application using [`cli_server`].
//! The app parses process arguments, dispatches them through a
//! [`default_interface`], and prints the response to stdout.
//!
//! Cards are tools that delegate rendering to the nearest render tool.
//! An empty-path card matches the root, serving as the default content
//! when no arguments are provided.
//!
//! ## Running the Example
//!
//! ```sh
//! # show root content
//! cargo run --example cli --features stack
//!
//! # show help for all routes
//! cargo run --example cli --features stack -- --help
//!
//! # navigate to a card
//! cargo run --example cli --features stack -- about
//!
//! # show help scoped to a subcommand
//! cargo run --example cli --features stack -- counter --help
//!
//! # not-found shows help for nearest ancestor
//! cargo run --example cli --features stack -- counter nonsense
//!
//! # directional navigation
//! cargo run --example cli --features stack -- --navigate=first-child
//! cargo run --example cli --features stack -- about --navigate=next-sibling
//! cargo run --example cli --features stack -- about --navigate=parent
//! ```
use beet::prelude::*;
mod petes_beets;

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), StackPlugin))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((cli_server(), petes_beets::stack()));
		})
		.run()
}
