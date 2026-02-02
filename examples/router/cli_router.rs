//! CLI router example.
//!
//! Demonstrates using `beet_router` with a CLI interface for testing routes
//! without starting an HTTP server.
//!
//! Run with:
//! ```sh
//! cargo run --example cli_router --features server_app -- foo
//! ```
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			RouterPlugin::default(),
		))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((
				CliServer,
				flow_exchange(|| {
					(InfallibleSequence, children![
						EndpointBuilder::get().with_action(|| {
							Response::ok_body("hello world", "text/plain")
						}),
						EndpointBuilder::get().with_path("foo").with_action(
							|| {
								Response::ok_body(
									"<div>hello foo</div>",
									"text/html",
								)
							},
						),
					])
				}),
			));
		})
		.run();
}
