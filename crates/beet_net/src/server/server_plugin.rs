// use crate::prelude::*;
use beet_core::prelude::*;
#[cfg(feature = "flow")]
use beet_flow::prelude::ControlFlowPlugin;

/// Plugin for running bevy servers.
/// by default this plugin will spawn the default [`HttpServer`] on [`Startup`]
#[derive(Default)]
pub struct ServerPlugin;


impl ServerPlugin {
	/// Runs the app with a tokio runtime if the `lambda` feature is enabled.
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
#[cfg(all(feature = "server", feature = "ureq", not(target_arch = "wasm32")))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
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
		time_ext::sleep_millis(50).await;
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
