//! A basic HTTP server using `handler_exchange`
//! to respond to all requests.
//!
//! Demonstrates application state and query params.
//!
//! Run with:
//! ```sh
//! cargo run --example server_minimal --features beet_net_server
//! ```
//!
//! Test with:
//! ```sh
//! curl http://localhost:8337?name=billy
//! ```
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
				HttpServer::default(),
				Count::default(),
				handler_exchange(|mut entity, request| {
					let name = request.get_param("name").unwrap_or("world");
					let mut count = entity.get_mut::<Count>().unwrap();
					count.0 += 1;

					let message = format!(
						"hello {}, you are visitor number {}",
						name, count.0
					);

					Response::ok_body(message, "text/plain")
				}),
			));
		})
		.run();
}

#[derive(Default, Component)]
struct Count(u32);
