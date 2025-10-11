use crate::prelude::*;
use beet_core::prelude::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering;



#[derive(Default)]
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
	fn build(&self, app: &mut App) { app.init_plugin::<AsyncPlugin>(); }
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

pub trait IntoHandlerFn<M> {
	fn into_handler_fn(self) -> HandlerFn;
}


pub struct AsyncWorldRequestIntoHandlerFn;
impl<Func, Fut, Out> IntoHandlerFn<(Out, AsyncWorldRequestIntoHandlerFn)>
	for Func
where
	Func: 'static + Send + Sync + Clone + FnOnce(AsyncWorld, Request) -> Fut,
	Fut: Send + Future<Output = Out>,
	Out: IntoResponse,
{
	fn into_handler_fn(self) -> HandlerFn {
		box_it(async move |entity, req| {
			self(entity.world(), req).await.into_response()
		})
	}
}
pub struct AsyncEntityRequestIntoHandlerFn;
impl<Func, Fut, Out> IntoHandlerFn<(Out, AsyncEntityRequestIntoHandlerFn)>
	for Func
where
	Func: 'static + Send + Sync + Clone + FnOnce(AsyncEntity, Request) -> Fut,
	Fut: Send + Future<Output = Out>,
	Out: IntoResponse,
{
	fn into_handler_fn(self) -> HandlerFn {
		box_it(async move |entity, req| self(entity, req).await.into_response())
	}
}
pub struct RequestIntoHandlerFn;
impl<Func, Fut, Out> IntoHandlerFn<(Out, RequestIntoHandlerFn)> for Func
where
	Func: 'static + Send + Sync + Clone + FnOnce(Request) -> Fut,
	Fut: Send + Future<Output = Out>,
	Out: IntoResponse,
{
	fn into_handler_fn(self) -> HandlerFn {
		box_it(async move |_, req| self(req).await.into_response())
	}
}

#[derive(Component)]
#[component(on_add=on_add)]
#[require(ServerStatus)]
pub struct Server {
	/// The port the server listens on
	pub port: u16,
	/// The function called by hyper for each request
	pub handler: HandlerFn,
}
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.run_system_cached_with(super::start_server, cx.entity);
}


impl Server {
	/// Create a new Server with an incrementing port to avoid
	/// collisions in tests
	pub fn new_test() -> Self {
		static PORT: AtomicU16 = AtomicU16::new(8340);
		Self {
			port: PORT.fetch_add(1, Ordering::SeqCst),
			handler: box_it(hello_server),
		}
	}


	pub fn local_url(&self) -> String {
		format!("http://127.0.0.1:{}", self.port)
	}

	pub fn with_handler<F, M>(mut self, func: F) -> Self
	where
		F: IntoHandlerFn<M>,
	{
		self.set_handler(func);
		self
	}

	pub fn set_handler<F, M>(&mut self, func: F) -> &mut Self
	where
		F: IntoHandlerFn<M>,
	{
		self.handler = func.into_handler_fn();
		self
	}

	pub fn handler(&self) -> HandlerFn { self.handler.clone() }
}

impl Default for Server {
	fn default() -> Self {
		Self {
			port: DEFAULT_SERVER_PORT,
			handler: box_it(hello_server),
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
	pub fn num_requests(&self) -> u128 { self.request_count }
	pub(super) fn increment_requests(&mut self) -> &mut Self {
		self.request_count += 1;
		self
	}
}

/// HTTP request handler that uses bevy's async world to manage state
async fn hello_server(entity: AsyncEntity, req: Request) -> Response {
	bevy::log::info!("Request: {} {}", req.method(), req.parts.uri.path());

	// Increment request counter using async world
	let count = entity
		.get::<ServerStatus, _>(|status| status.num_requests())
		.await;

	let response_text = format!("greetings! Request #{}", count);

	// Create our Response and convert it back to hyper response
	Response::ok_body(response_text, "text/plain")
}
