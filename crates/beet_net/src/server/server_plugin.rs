//! Plugin and utilities for running Bevy-based HTTP servers.
use crate::prelude::*;
use beet_core::prelude::*;

/// Plugin for running Bevy HTTP servers.
///
/// Sets up the async runtime needed for action-based exchange handling
/// and registers reflection for the server component types.
#[derive(Default)]
pub struct ServerPlugin;


impl Plugin for ServerPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>()
			.register_type::<Server>()
			.register_type::<ServerKind>()
			.register_type::<CliServer>()
			.register_type::<HttpServer>();

		// the backend registry the `Server` orchestrator selects against. The two
		// `beet_net` backends register here; downstream crates (eg `beet_router`'s
		// TUI) register their own kinds.
		let mut backends = app.world_mut().get_resource_or_init::<ServerBackends>();
		backends.register(ServerKind::Cli, ServerBackendEntry {
			is_present: |entity| entity.contains::<CliServer>(),
			start: CliServer::start,
		});
		backends.register(ServerKind::Http, ServerBackendEntry {
			is_present: |entity| entity.contains::<HttpServer>(),
			start: HttpServer::start,
		});

		// per-request logging: log each exchange's method/path/status/duration on
		// completion. std-only, since [`ExchangeStats`] (the request counter the
		// observer increments) backs the std [`HttpServer`] requirement.
		#[cfg(feature = "std")]
		app.add_observer(exchange_stats);
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
				.spawn((
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
