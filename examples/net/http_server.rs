//! # Basic Server
//!
//! This example demonstrates creating a basic server in Beet
//! using the [`handler_exchange`], which is the least opinionated exchange pattern
//! and can only process a single request at a time.
//! For concurrent request handling see [`spawn_exchange`].
//!
//!
//! ## Running the Example
//!
//! ```sh
//! cargo run --example server --features http_server
//! # or with CliServer
//! cargo run --example server --features http_server -- --name=billy
//! ```
//!
//! Then test it with:
//! ```sh
//! curl http://localhost:8337
//! curl http://localhost:8337?name=billy
//! ```
//!
//! Try refreshing multiple times to see the visitor count increment!
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

/// Handler function that processes all incoming requests.
fn handler(mut server: EntityWorldMut, request: Request) -> Response {
	// only accept `/` routes
	if !request.path().is_empty() {
		let message = format!("Not Found: {}", request.path_string());
		println!(
			"{}: {} - Not Found",
			request.method(),
			request.path_string()
		);
		return Response::from_status_body(
			StatusCode::NotFound,
			message,
			"text/plain",
		);
	}

	// increment visitor count
	let name = request.get_param("name").unwrap_or("world");

	// increment visitor count
	let mut count = server.get_mut::<Count>().unwrap();
	count.0 += 1;

	let message = format!(
		r#"
hello {}
you are visitor number {}

pass the 'name' parameter to receive a warm personal greeting.
"#,
		name, count.0
	);

	println!("{}: {}", request.method(), request.path_string());
	Response::ok_body(message, "text/plain")
}
