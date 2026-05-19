//! # CLI Router Example
//!
//! The simplest possible router: a CLI server with two text routes and
//! a [`Script`]-backed route that transforms a request struct into a
//! greeting.
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
//! # invoke the scripted greeter
//! cargo run --example cli -- greet --name=world
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
	]));
}
