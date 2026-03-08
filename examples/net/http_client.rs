//! HTTP client example
//!
//! This example demonstrates how to make HTTP requests using the beet HTTP client.
//! It pings `example.com` and verifies the response.
//!
//! Run with:
//! ```sh
//! cargo run --example client --features net,ureq,native-tls
//! ```

use beet::net::prelude::*;
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((MinimalPlugins, AsyncPlugin::default()))
		.add_systems(Startup, ping_example)
		.run();
	println!("Done");
}

fn ping_example(mut async_commands: AsyncCommands) {
	async_commands.run(|world| async move {
		let response = Request::get("http://example.com")
			.send()
			.await
			.expect("Failed to send request");

		assert!(response.status().is_ok());
		let body = response.text().await.expect("Failed to read response body");

		assert!(body.contains("Example Domain"));

		world.write_message(AppExit::Success);
	});
}
