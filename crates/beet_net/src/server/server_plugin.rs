//! Plugin and utilities for running Bevy-based HTTP servers.
// use crate::prelude::*;
use beet_core::prelude::*;

/// Plugin for running Bevy HTTP servers.
///
/// Sets up the async runtime needed for tool-based exchange handling.
#[derive(Default)]
pub struct ServerPlugin;


impl Plugin for ServerPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>();
		// .add_observer(exchange_stats);
	}
}

#[cfg(test)]
#[cfg(all(
	feature = "server",
	feature = "ureq",
	not(feature = "lambda"),
	not(target_arch = "wasm32")
))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	// #[ignore = "flaky with all features?"]
	async fn http_server() {
		let server = HttpServer::new_test(start_mini_http_server_with_tcp);
		let url = server.0.local_url();
		let _handle = std::thread::spawn(|| {
			App::new()
				.add_plugins((MinimalPlugins, ServerPlugin))
				.spawn_then((
					server,
					exchange_handler(|_| Response::ok().with_body("hello")),
				))
				.run();
		});
		time_ext::sleep_millis(200).await;
		for _ in 0..10 {
			Request::post(&url)
				.send()
				.await
				.unwrap()
				.into_result()
				.await
				.unwrap()
				.text()
				.await
				.unwrap()
				.xpect_eq("hello");
		}
	}
}
