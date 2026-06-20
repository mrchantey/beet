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
	commands
		.spawn((
			CliServer::default(),
			(default_router(), children![
			exchange_route(
				"",
				Action::<(), &str>::new_pure(|_| { "hello world" })
			),
			exchange_route(
				"foo",
				Action::<(), &str>::new_pure(|_| { "hello foo" })
			),
			// a `Script` is pure data, so pair it with an `ExchangeScript` to
			// make the entity a dispatchable route.
			(
				Script::<QueryParams<GreetRequest>, String>::rhai(
					r#""hello " + input.name"#,
				),
				ExchangeScript::<QueryParams<GreetRequest>, String, _, _>::default(),
				PathPartial::new("greet"),
			),
			// same idea, but the script receives the full [`RequestParts`]
			// and digs out the `name` query parameter itself.
			(
				Script::<RequestParts, String>::rhai(
					r#""hello " + input.url.params.name[0]"#,
				),
				ExchangeScript::<RequestParts, String, _, _>::default(),
				PathPartial::new("greet-request"),
			),
		]),
		))
		.trigger(ActionIn::boot);
}
