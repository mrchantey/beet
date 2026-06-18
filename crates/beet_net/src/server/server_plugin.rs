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
			// the process-lifetime refcount the servers and the binary's load scope
			// claim, driving exit when it returns to zero.
			.init_resource::<KeepAlive>()
			.register_type::<CliServer>()
			.register_type::<HttpServer>()
			// the markup boot verb, so a `<Router {(.., ServeOnLoad)}>` entry
			// resolves it.
			.register_type::<ServeOnLoad>();

		// exit once nothing keeps the process alive: a finished `CliServer`
		// exchange or a stopped server drops its `KeepAliveGuard`, and when the last
		// claim goes the refcount reaches zero. Gated on a `KeepAlive` change so
		// there is no per-frame polling. Lives here (not in a binary's main) so every
		// server app — the cli and the examples alike — exits cleanly.
		app.add_systems(
			Last,
			exit_when_unclaimed.run_if(resource_exists_and_changed::<KeepAlive>),
		);

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

/// `Last`: emit [`AppExit::Success`] once the [`KeepAlive`] refcount reaches zero,
/// so a process with no remaining claim (every server stopped, the cli exchange
/// finished) exits cleanly. A long-running server holds a guard, so it persists.
fn exit_when_unclaimed(
	keep_alive: Res<KeepAlive>,
	mut exit: MessageWriter<AppExit>,
) {
	if keep_alive.count() == 0 {
		exit.write(AppExit::Success);
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
