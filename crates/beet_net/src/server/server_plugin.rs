//! Plugin and utilities for running Bevy-based HTTP servers.
use crate::prelude::*;
use beet_core::prelude::*;

/// Plugin for running Bevy HTTP servers.
///
/// Sets up the async runtime needed for action-based exchange handling,
/// registers reflection for the server component and event types, and installs
/// the compile-time-selected [`HttpServer`] backend via [`set_http_server`].
#[derive(Default)]
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>()
			.register_type::<CliServer>()
			.register_type::<HttpServer>()
			// the markup load verb (and its opt-out), so a
			// `<Router {(.., BootOnLoad)}>` entry resolves them.
			.register_type::<BootOnLoad>()
			.register_type::<ExchangeOnLoad>()
			.register_type::<DisableBootOnLoad>()
			// the boot<->exchange bridges, markup-spawnable so an entry can wire a
			// boot into its request pipeline or boot another entry from a route.
			.register_type::<BootToExchange>()
			.register_type::<ExchangeToBoot>();

		// the process exits when `boot` writes `AppExit` for the one-shot it
		// resolves; a long-running server never resolves its boot call, so its
		// parked `Running<Response>` holds the run open with no refcount.

		// install the HTTP backend `HttpServer` invokes on start. The cascade
		// mirrors the old per-component dispatch, now in one place: a downstream
		// `set_http_server` (an embassy / esp adapter) replaces it on no_std,
		// where no feature backend is compiled in. Skipped in `beet_net`'s own
		// tests, which install a stub hook per case. `set_http_server` errors if
		// one is already installed, so ignore a re-install across plugin adds.
		#[cfg(not(test))]
		{
			cfg_if! {
				if #[cfg(all(feature = "lambda", not(target_arch = "wasm32")))] {
					set_http_server(|entity, shutdown| Box::pin(super::start_lambda_server(entity, shutdown))).ok();
				} else if #[cfg(all(feature = "hyper", not(target_arch = "wasm32")))] {
					set_http_server(|entity, shutdown| Box::pin(super::start_hyper_server(entity, shutdown))).ok();
				} else if #[cfg(all(feature = "server", not(target_arch = "wasm32")))] {
					set_http_server(|entity, shutdown| Box::pin(super::start_mini_http_server(entity, shutdown))).ok();
				} else {
					// no feature backend: a downstream `set_http_server` supplies one.
				}
			}
		}

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
					exchange_handler(|_| {
						Response::ok().with_body("hello")
					}),
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
