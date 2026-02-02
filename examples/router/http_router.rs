//! HTTP router example.
//!
//! Demonstrates a basic HTTP router using `beet_router` for request routing.
//!
//! Run with:
//! ```sh
//! cargo run --example http_router --features server
//! ```
//!
//! Test with:
//! ```sh
//! curl http://localhost:5000
//! curl http://localhost:5000/foo
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
				HttpServer::new(5000),
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
