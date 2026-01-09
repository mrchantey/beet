use crate::prelude::*;
use beet_core::prelude::*;
#[cfg(feature = "flow")]
use beet_flow::prelude::ControlFlowPlugin;

/// Represents a http request, may contain a [`Request`] or [`Response`]
#[derive(Default, Component)]
pub struct Exchange;

/// Points to the [`HttpServer`] that this exchange was spawned by.
/// We don't use [`Children`] because some server patterns have a different
/// meaning for that, for example `beet_router` uses `beet_flow` to represent
/// the routes, and the [`Exchange`] is an `agent`.
#[derive(Deref, Component)]
#[relationship(relationship_target = Exchanges)]
#[require(Exchange)]
pub struct ExchangeOf(pub Entity);

/// List of [`Exchange`]
#[derive(Deref, Component)]
#[relationship_target(relationship = ExchangeOf, linked_spawn)]
pub struct Exchanges(Vec<Entity>);

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
		app.init_plugin::<AsyncPlugin>().add_observer(server_stats);
		#[cfg(feature = "flow")]
		app.init_plugin::<ControlFlowPlugin>();
	}
}


/// Update server stats if available
fn server_stats(
	ev: On<Insert, Response>,
	mut servers: Query<&mut ServerStatus>,
	exchange: Query<(&RequestMeta, &Response, &ExchangeOf)>,
) -> Result {
	let entity = ev.event_target();
	let Ok((meta, response, exchange_of)) = exchange.get(entity) else {
		return Ok(());
	};
	let status = response.status();
	let duration = meta.started().elapsed();
	let path = meta.path_string();
	let method = meta.method();

	let Ok(mut stats) = servers.get_mut(exchange_of.get()) else {
		return Ok(());
	};

	bevy::log::info!(
		"
Request Complete
  path:     {}
  method:   {}
  duration: {}
  status:   {}
  index:    {}
",
		path,
		method,
		time_ext::pretty_print_duration(duration),
		status,
		stats.request_count()
	);
	stats.increment_requests();
	Ok(())
}
#[derive(Clone, Component)]
#[component(on_add=on_add_http)]
#[require(ExchangeSpawner, ServerStatus)]
pub struct HttpServer {
	/// The port the server listens on. This may be updated at runtime,
	/// for instance if the provided port is `0` it may be updated to
	/// some random available port by the os like `98304`.
	/// This is ignored by lambda_server
	pub port: u16,
}


// using commands allows a ServerHandler to be inserted, instead of running immediately
// and using the one inserted via Required.
#[allow(unused)]
fn on_add_http(mut world: DeferredWorld, cx: HookContext) {
	#[cfg(all(feature = "lambda", not(target_arch = "wasm32")))]
	world
		.commands()
		.run_system_cached_with(super::start_lambda_server, cx.entity);

	#[cfg(all(
		feature = "server",
		not(feature = "lambda"),
		not(target_arch = "wasm32")
	))]
	world
		.commands()
		.run_system_cached_with(super::start_hyper_server, cx.entity);

	#[cfg(not(any(
		all(feature = "server", not(target_arch = "wasm32")),
		all(feature = "lambda", not(target_arch = "wasm32"))
	)))]
	panic!(
		"The ServerPlugin can only be used on non-wasm32 targets with the `server` or `lambda` feature enabled"
	);
}


impl HttpServer {
	/// Create a new Server with an incrementing port to avoid
	/// collisions in tests
	pub fn new_test() -> Self {
		use std::sync::atomic::AtomicU16;
		use std::sync::atomic::Ordering;
		static PORT: AtomicU16 = AtomicU16::new(DEFAULT_SERVER_TEST_PORT);
		Self {
			port: PORT.fetch_add(1, Ordering::SeqCst),
			..default()
		}
	}

	pub fn local_url(&self) -> String {
		format!("http://127.0.0.1:{}", self.port)
	}
}

impl Default for HttpServer {
	fn default() -> Self {
		Self {
			port: DEFAULT_SERVER_PORT,
		}
	}
}


#[derive(Default, Component)]
pub struct ServerStatus {
	request_count: u128,
}
impl ServerStatus {
	pub fn request_count(&self) -> u128 { self.request_count }
	pub(super) fn increment_requests(&mut self) -> &mut Self {
		self.request_count += 1;
		self
	}
}

#[cfg(test)]
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[sweet::test]
	async fn http_server() {
		let server = HttpServer::new_test();
		let url = server.local_url();
		let _handle = std::thread::spawn(|| {
			App::new()
				.add_plugins((MinimalPlugins, ServerPlugin))
				.spawn_then((
					server,
					ExchangeSpawner::new_handler(|_, _| {
						Response::ok().with_body("hello")
					}),
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
