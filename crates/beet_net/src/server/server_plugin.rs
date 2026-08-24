//! Plugin and utilities for running Bevy-based HTTP servers.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Plugin for running Bevy HTTP servers.
///
/// Sets up the async runtime needed for action-based exchange handling,
/// registers reflection for the server component and event types, and installs
/// the compile-time-selected [`HttpServer`] backend via [`HttpServer::set_backend`].
#[derive(Default)]
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>()
			.register_type::<CliServer>()
			.register_type::<HttpServer>()
			.register_type::<Tls>()
			// the markup load verb, so an `<HttpServer>` entry, a `{CallOnLoad}`
			// script or behaviour scene resolves it.
			.register_type::<CallOnLoad>()
			// a boot whose `--server` selected nothing fails rather than parking
			.add_observer(assert_server_started);

		// the process exits when `CallOnLoad` writes `AppExit` for the one-shot
		// it resolves; a long-running server never resolves its boot call, so its
		// parked `Running<Response>` holds the run open with no refcount.

		// install the HTTP backend `HttpServer` invokes on start. The cascade
		// mirrors the old per-component dispatch, now in one place: a downstream
		// `HttpServer::set_backend` (an embassy / esp adapter) replaces it on no_std,
		// where no feature backend is compiled in. Skipped in `beet_net`'s own
		// tests, which install a stub hook per case. `HttpServer::set_backend` errors if
		// one is already installed, so ignore a re-install across plugin adds.
		#[cfg(not(test))]
		{
			// the lambda backend serves only inside an actual Lambda task (its
			// runtime sets AWS_LAMBDA_FUNCTION_NAME); anywhere else a
			// lambda-featured binary (eg an --all-features install) falls
			// through to the standard backends below. Installed *first*, so in a
			// Lambda task the first-write-wins `HttpServer::set_backend` (an `OnceLock`)
			// keeps lambda: the hyper/mini install below then no-ops (its `.ok()`
			// swallows the already-installed error), so hyper never clobbers it.
			#[cfg(all(feature = "lambda", not(target_arch = "wasm32")))]
			if env_ext::var("AWS_LAMBDA_FUNCTION_NAME").is_ok() {
				HttpServer::set_backend(|entity, shutdown| {
					Box::pin(HttpServer::start_lambda(entity, shutdown))
				})
				.ok();
			}
			cfg_if! {
				if #[cfg(all(feature = "hyper", not(target_arch = "wasm32")))] {
					HttpServer::set_backend(|entity, shutdown| Box::pin(HttpServer::start_hyper(entity, shutdown))).ok();
				} else if #[cfg(all(feature = "server", not(target_arch = "wasm32")))] {
					HttpServer::set_backend(|entity, shutdown| Box::pin(HttpServer::start_mini(entity, shutdown))).ok();
				} else {
					// no feature backend: a downstream `HttpServer::set_backend` supplies one.
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

/// A boot that declared servers but started none has nothing to hold the process
/// open, so fail the parked call rather than park forever: `beet serve site
/// --server=nonexistent` exits with the error instead of hanging.
///
/// The thin server-layer reading of a [`RunningSet`]'s tally. It resolves the
/// call in the same breath as the walk, so no marker component and no
/// frame-later sweep are needed.
fn assert_server_started(
	ev: On<RunningSetStarted>,
	sets: Query<(), With<RunningSet<Request, Response>>>,
	mut commands: Commands,
) {
	if ev.started > 0 || !sets.contains(ev.entity) {
		return;
	}
	commands
		.entity(ev.entity)
		.queue(FailRun::<Response>::new(bevyhow!(
			"No server started for {}, does --server (or BEET_SERVER) name a \
			 server this entry declares?",
			ev.entity
		)));
}

#[cfg(test)]
mod boot_check_test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A `--server` naming nothing this entry declares must exit, not park the
	/// process on a `Running` no server will ever resolve.
	#[beet_core::test]
	async fn unselected_boot_exits() {
		crate::server::http_server::stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = app.world_mut().spawn(HttpServer::new(0)).id();
		app.world_mut().entity_mut(entity).run_async_local(
			|server| async move {
				CallOnLoad::call(
					server,
					Request::from_cli_str("--server=nonexistent"),
				)
				.await
			},
		);
		app.run_async().await.xpect_eq(AppExit::error());
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
		let server = HttpServer::new_test(HttpServer::start_mini_with_tcp);
		let url = server.0.local_url();
		let _handle = std::thread::spawn(|| {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, ServerPlugin));
			// the server owns the boot, its dispatch host is the child
			app.world_mut()
				.spawn((server, children![exchange_ext::handler(|_| {
					Response::ok().with_body("hello")
				})]));
			app.run();
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
