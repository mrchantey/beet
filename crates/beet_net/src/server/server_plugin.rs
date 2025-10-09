use crate::prelude::*;
use crate::server::run_server;
use beet_core::prelude::*;
use std::pin::Pin;
use std::sync::Arc;



pub struct ServerPlugin;

impl Plugin for ServerPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin(AsyncPlugin)
			.init_resource::<ServerSettings>()
			.init_resource::<ServerStatus>()
			.add_systems(Startup, run_server);
	}
}

pub(super) type HandlerFn = Arc<
	Box<
		dyn 'static
			+ Send
			+ Sync
			+ Fn(
				AsyncWorld,
				Request,
			) -> Pin<Box<dyn Send + Future<Output = Response>>>,
	>,
>;

#[derive(Resource)]
pub struct ServerSettings {
	pub port: u16,
	pub handler: HandlerFn,
}

impl ServerSettings {
	pub fn default_local_url() -> String {
		format!("http://127.0.0.1:{DEFAULT_SERVER_PORT}")
	}

	pub fn with_handler<Func, Fut>(mut self, func: Func) -> Self
	where
		Func:
			'static + Send + Sync + Clone + FnOnce(AsyncWorld, Request) -> Fut,
		Fut: Send + Future<Output = Response>,
	{
		self.handler = box_it(func);
		self
	}
	pub fn handler(&self) -> HandlerFn { self.handler.clone() }
}

impl Default for ServerSettings {
	fn default() -> Self {
		Self {
			port: DEFAULT_SERVER_PORT,
			handler: box_it(hello_server),
		}
	}
}

fn box_it<Func, Fut>(func: Func) -> HandlerFn
where
	Func: 'static + Send + Sync + Clone + FnOnce(AsyncWorld, Request) -> Fut,
	Fut: Send + Future<Output = Response>,
{
	Arc::new(Box::new(move |world, request| {
		let func = func.clone();
		Box::pin(async move { func.clone()(world, request).await })
	}))
}

#[derive(Default, Resource)]
pub struct ServerStatus {
	request_count: u128,
}
impl ServerStatus {
	pub fn num_requests(&self) -> u128 { self.request_count }
	pub(super) fn increment_requests(&mut self) -> &mut Self {
		self.request_count += 1;
		self
	}
}

/// HTTP request handler that uses bevy's async world to manage state
async fn hello_server(world: AsyncWorld, req: Request) -> Response {
	bevy::log::info!("Request: {} {}", req.method(), req.parts.uri.path());

	// Increment request counter using async world
	let count = world
		.with_resource_then::<ServerStatus, _>(|mut status| {
			status.increment_requests().num_requests()
		})
		.await;

	let response_text = format!("Hello from Bevy! Request #{}", count);

	// Create our Response and convert it back to hyper response
	Response::ok_body(response_text, "text/plain")
}
