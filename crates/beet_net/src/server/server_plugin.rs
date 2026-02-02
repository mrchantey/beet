//! Plugin and utilities for running Bevy-based HTTP servers.
// use crate::prelude::*;
use beet_core::prelude::*;
#[cfg(feature = "flow")]
use beet_flow::prelude::ControlFlowPlugin;

/// Plugin for running Bevy HTTP servers.
///
/// Sets up the async runtime and optionally integrates with `beet_flow`
/// for behavior tree-based request handling.
#[derive(Default)]
pub struct ServerPlugin;


impl ServerPlugin {
	/// Runs the app with the appropriate async runtime.
	///
	/// - With `lambda` feature: Uses a multi-threaded Tokio runtime
	/// - Otherwise: Uses Bevy's default schedule runner
	pub fn maybe_tokio_runner(mut app: App) -> AppExit {
		#[cfg(all(feature = "lambda", not(target_arch = "wasm32")))]
		{
			tokio::runtime::Builder::new_multi_thread()
				.enable_all()
				.build()
				.unwrap()
				.block_on(app.run_async())
		}
		#[cfg(not(all(feature = "lambda", not(target_arch = "wasm32"))))]
		{
			// just use default runner
			use bevy::app::ScheduleRunnerPlugin;
			ScheduleRunnerPlugin::default().build(&mut app);
			app.run()
		}
	}
}

impl Plugin for ServerPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>();
		// .add_observer(exchange_stats);
		#[cfg(feature = "flow")]
		app.init_plugin::<ControlFlowPlugin>();
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
		let server = HttpServer::new_test();
		let url = server.local_url();
		let _handle = std::thread::spawn(|| {
			App::new()
				.add_plugins((MinimalPlugins, ServerPlugin))
				.spawn_then((
					server,
					handler_exchange(|_, _| Response::ok().with_body("hello")),
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
