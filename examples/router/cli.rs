//! # CLI Router Example
//!
//! The simplest possible router: a CLI server with two text routes and
//! two [`Script`]-backed routes — one that extracts a typed query
//! struct, and one that runs against the full [`RequestParts`].
//!
//! ## Running the Example
//!
//! ```sh
//! # visit the home route
//! cargo run --example cli
//!
//! # visit the /foo route
//! cargo run --example cli -- foo
//!
//! # invoke the scripted greeter via a typed query struct
//! cargo run --example cli -- greet --name=world
//!
//! # invoke the scripted greeter via the raw request parts
//! cargo run --example cli -- greet-request --name=world
//! ```
use beet::prelude::*;
use serde::Deserialize;
use serde::Serialize;

fn main() -> AppExit {
	App::new()
		.add_plugins((MinimalPlugins, ClientAppPlugin))
		.add_systems(Startup, setup)
		.run()
}

/// Query params for the scripted greet route, exposed to the rhai
/// script as `input.name`.
#[derive(Serialize, Deserialize)]
struct GreetRequest {
	name: String,
}

fn setup(mut commands: Commands) {
	commands.spawn((CliServer::default(), router(), children![
		exchange_route("", Action::<(), &str>::new_pure(|_| { "hello world" })),
		exchange_route("foo", Action::<(), &str>::new_pure(|_| { "hello foo" })),
		exchange_route(
			"greet",
			Script::<QueryParams<GreetRequest>, String>::rhai(
				r#""hello " + input.name"#,
			),
		),
		// same idea, but the script receives the full [`RequestParts`]
		// and digs out the `name` query parameter itself.
		exchange_route(
			"greet-request",
			Script::<RequestParts, String>::rhai(
				r#""hello " + input.url.params.name[0]"#,
			),
		),
	]));
}
