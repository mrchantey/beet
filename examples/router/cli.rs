//! # CLI Router Example
//!
//! The simplest possible router: a CLI server with two text routes.
//!
//! ## Running the Example
//!
//! ```sh
//! # visit the home route
//! cargo run --example cli
//!
//! # visit the /foo route
//! cargo run --example cli -- foo
//! ```
use beet::prelude::*;

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, ClientAppPlugin))
		.add_systems(Startup, setup)
		.run()
}

fn setup(mut commands: Commands) {
	commands.spawn((CliServer::default(), router(), children![
		exchange_route("", Action::<(), &str>::new_pure(|_| { "hello world" })),
		exchange_route(
			"foo",
			Action::<(), &str>::new_pure(|_| { "hello foo" })
		),
	]));
}
