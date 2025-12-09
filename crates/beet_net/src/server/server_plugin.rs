use crate::prelude::*;
use beet_core::prelude::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering;

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
pub struct ServerPlugin {
	/// Spawn the server on add
	pub spawn_server: Option<HttpServer>,
}


impl ServerPlugin {
	/// Create a new ServerPlugin that does not spawn a server
	pub fn without_server(mut self) -> Self {
		self.spawn_server = None;
		self
	}
	pub fn with_server(server: HttpServer) -> Self {
		Self {
			spawn_server: Some(server),
			..default()
		}
	}
}
impl Default for ServerPlugin {
	fn default() -> Self {
		Self {
			spawn_server: Some(HttpServer::default()),
		}
	}
}

impl Plugin for ServerPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>().add_observer(exchange_meta);
		if let Some(server) = &self.spawn_server {
			let server = server.clone();
			app.add_systems(Startup, move |mut commands: Commands| {
				commands.spawn(server.clone());
			});
			// app.world_mut().spawn(server.clone());
		}
	}
}

pub(super) type HandlerFn = Arc<
	Box<
		dyn 'static
			+ Send
			+ Sync
			+ Fn(
				AsyncEntity,
				Request,
			) -> Pin<Box<dyn Send + Future<Output = Response>>>,
	>,
>;


#[derive(Clone, Component)]
#[component(on_add=on_add)]
#[require(ServerStatus)]
pub struct HttpServer {
	/// The port the server listens on
	pub port: u16,
	/// The function called by hyper for each request
	pub handler: HandlerFn,
}
#[allow(unused)]
fn on_add(mut world: DeferredWorld, cx: HookContext) {
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
		static PORT: AtomicU16 = AtomicU16::new(DEFAULT_SERVER_TEST_PORT);
		Self {
			port: PORT.fetch_add(1, Ordering::SeqCst),
			..default()
		}
	}


	pub fn local_url(&self) -> String {
		format!("http://127.0.0.1:{}", self.port)
	}

	pub fn with_handler<F, Fut>(mut self, func: F) -> Self
	where
		F: 'static + Send + Sync + Clone + FnOnce(AsyncEntity, Request) -> Fut,
		Fut: Send + Future<Output = Response>,
	{
		self.set_handler(func);
		self
	}

	pub fn set_handler<F, Fut>(&mut self, func: F) -> &mut Self
	where
		F: 'static + Send + Sync + Clone + FnOnce(AsyncEntity, Request) -> Fut,
		Fut: Send + Future<Output = Response>,
	{
		self.handler = box_it(func);
		self
	}

	pub fn handler(&self) -> HandlerFn { self.handler.clone() }
}

impl Default for HttpServer {
	fn default() -> Self {
		Self {
			port: DEFAULT_SERVER_PORT,
			handler: box_it(default_handler),
		}
	}
}

fn box_it<Func, Fut>(func: Func) -> HandlerFn
where
	Func: 'static + Send + Sync + Clone + FnOnce(AsyncEntity, Request) -> Fut,
	Fut: Send + Future<Output = Response>,
{
	Arc::new(Box::new(move |world, request| {
		let func = func.clone();
		Box::pin(async move { func.clone()(world, request).await })
	}))
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
