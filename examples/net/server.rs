//! A basic HTTP server using `handler_exchange`
//! to respond to all requests.
//!
//! Demonstrates application state and query params.
//!
//! Run with:
//! ```sh
//! cargo run --example server --features http_server
//! # or with CliServer
//! cargo run --example server --features http_server -- --name=billy
//! ```
//!
//! Test with:
//! ```sh
//! curl http://localhost:8337?name=billy
//! ```
//!
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			ServerPlugin::default(),
		))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn((
				// CliServer::default(),
				HttpServer::default(),
				Count::default(),
				handler_exchange(handler),
			));
		})
		.run();
}

#[derive(Default, Component)]
struct Count(u32);

fn handler(mut entity: EntityWorldMut, request: Request) -> Response {
	let name = request.get_param("name").unwrap_or("world");
	let mut count = entity.get_mut::<Count>().unwrap();
	count.0 += 1;

	let message = format!("hello {}, you are visitor number {}", name, count.0);

	println!("{}: {}", request.method(), request.path_string());
	Response::ok_body(message, "text/plain")
}
