use std::convert::Infallible;

use crate::prelude::*;
use axum::Router;
use axum::extract::FromRequest;
use axum::handler::Handler;
use axum::response::IntoResponse;
use axum::routing;
use axum::routing::Route;
use beet_core::net::RouteInfo;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use tower::Layer;
use tower::Service;


// pub struct Route<H,M>{
// 	handler:H,
// 	marker: PhantomData<M>,
// }

pub struct StatefulAppRouter<S> {
	pub state: S,
	pub app: App,
}

impl<S> std::ops::Deref for StatefulAppRouter<S> {
	type Target = App;
	fn deref(&self) -> &Self::Target { &self.app }
}
impl<S> std::ops::DerefMut for StatefulAppRouter<S> {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.app }
}

impl<S> AxumRouterExt for StatefulAppRouter<S>
where
	S: DerivedAppState,
{
	type State = S;
	fn app(&self) -> &App { &self.app }
	fn app_mut(&mut self) -> &mut App { &mut self.app }
	fn take_app(self) -> App { self.app }
	fn take_router(&mut self) -> Router<()> {
		self.app_mut()
			.world_mut()
			.remove_non_send_resource::<Router<Self::State>>()
			.unwrap_or_default()
			.with_state(self.state.clone())
	}
}
impl AxumRouterExt for App {
	type State = AppRouterState;
	fn app(&self) -> &App { self }
	fn app_mut(&mut self) -> &mut App { self }
	fn take_app(self) -> App { self }
	fn take_router(&mut self) -> Router<()> {
		self.app_mut()
			.world_mut()
			.remove_non_send_resource::<Router<Self::State>>()
			.unwrap_or_default()
			.with_state(AppRouterState::default())
	}
}


pub trait AxumRouterExt: Sized {
	type State: DerivedAppState;
	fn app(&self) -> &App;
	fn app_mut(&mut self) -> &mut App;
	fn take_app(self) -> App;
	fn take_router(&mut self) -> Router<()>;
	/// Drop the current state, if any, and return a new router with the given state.
	/// As per axum conventions any routes previously added will still access the old state.
	fn with_state(self, state: Self::State) -> StatefulAppRouter<Self::State> {
		StatefulAppRouter {
			state,
			app: self.take_app(),
		}
	}

	/// Access the stored router, make changes to it, and return the updated router.
	/// If no router is stored a new one is created.
	fn router(
		&mut self,
		func: impl FnOnce(Router<Self::State>) -> Router<Self::State>,
	) -> &mut Self {
		let mut router = self
			.app_mut()
			.world_mut()
			.remove_non_send_resource::<Router<Self::State>>()
			.unwrap_or_default();
		router = func(router);
		self.app_mut().world_mut().insert_non_send_resource(router);
		self
	}


	/// Accepts a bevy system with [`FromRequest`] extractors as its first argument,
	/// returning an [IntoResponse]
	fn add_route<
		Extract,
		ExtractMarker,
		Response,
		Handler,
		HandlerMarker,
		PluginType,
	>(
		&mut self,
		info: impl Into<RouteInfo>,
		handler: Handler,
		plugin: PluginType,
	) -> &mut Self
	where
		Self: Sized,
		Self::State: DerivedAppState,
		Extract: 'static + Send + Sync + SystemInput,
		ExtractMarker: 'static + Send + Sync,
		// Extract::Inner<'static>:
		Extract::Inner<'static>:
			'static + Send + Sync + FromRequest<Self::State, ExtractMarker>,
		Response: 'static + Send + Sync + IntoResponse,
		Handler: 'static
			+ Send
			+ Sync
			+ Clone
			+ IntoSystem<Extract, Response, HandlerMarker>,
		HandlerMarker: 'static + Send + Sync,
		PluginType: ClonePlugin + 'static + Send + Sync,
	{
		let route_info = info.into();
		self.router(|router| {
			let container = ClonePluginContainer::new(plugin);

			let route_handler =
				async move |state: axum::extract::State<Self::State>,
				            extract: Extract::Inner<'static>|
				            -> AppResult<Response> {
					let handler = handler.clone();
					let plugin = container.clone();
					let mut app = state.create_app();
					plugin.add_to_app(&mut app);
					app.run_once();
					let result = app
						.world_mut()
						.run_system_once_with(handler, extract)?;
					app.update();
					Ok(result)
				};

			router.route(
				&route_info.path.to_string_lossy(),
				routing::on(
					route_info.method.into_axum_method(),
					route_handler,
				),
			)
		});
		self
	}

	/// ---- from bevy_webserver ---- ///

	fn route_service<T>(&mut self, path: &str, service: T) -> &mut Self
	where
		T: Service<axum::extract::Request, Error = Infallible>
			+ Clone
			+ Send
			+ Sync
			+ 'static,
		T::Response: IntoResponse,
		T::Future: Send + 'static,
	{
		self.router(move |router| router.route_service(path, service));
		self
	}

	fn nest(&mut self, path: &str, router2: Router<Self::State>) -> &mut Self {
		self.router(move |router| router.nest(path, router2));
		self
	}

	fn nest_service<T>(&mut self, path: &str, service: T) -> &mut Self
	where
		T: Service<axum::extract::Request, Error = Infallible>
			+ Clone
			+ Send
			+ Sync
			+ 'static,
		T::Response: IntoResponse,
		T::Future: Send + 'static,
	{
		self.router(|router| router.nest_service(path, service));
		self
	}

	fn merge<R>(&mut self, other: R) -> &mut Self
	where
		R: Into<Router<Self::State>>,
	{
		self.router(|router| router.merge(other));
		self
	}

	fn layer<L>(&mut self, layer: L) -> &mut Self
	where
		L: Layer<Route> + Clone + Send + Sync + 'static,
		L::Service:
			Service<axum::extract::Request> + Clone + Send + Sync + 'static,
		<L::Service as Service<axum::extract::Request>>::Response:
			IntoResponse + 'static,
		<L::Service as Service<axum::extract::Request>>::Error:
			Into<Infallible> + 'static,
		<L::Service as Service<axum::extract::Request>>::Future: Send + 'static,
	{
		self.router(|router| router.layer(layer));
		self
	}

	fn route_layer<L>(&mut self, layer: L) -> &mut Self
	where
		L: Layer<Route> + Clone + Send + Sync + 'static,
		L::Service:
			Service<axum::extract::Request> + Clone + Send + Sync + 'static,
		<L::Service as Service<axum::extract::Request>>::Response:
			IntoResponse + 'static,
		<L::Service as Service<axum::extract::Request>>::Error:
			Into<Infallible> + 'static,
		<L::Service as Service<axum::extract::Request>>::Future: Send + 'static,
	{
		self.router(|router| router.layer(layer));
		self
	}

	fn fallback<H, T>(&mut self, handler: H) -> &mut Self
	where
		H: Handler<T, Self::State>,
		T: 'static,
	{
		self.router(|router| router.fallback(handler));
		self
	}

	fn fallback_service<T>(&mut self, service: T) -> &mut Self
	where
		T: Service<axum::extract::Request, Error = Infallible>
			+ Clone
			+ Send
			+ Sync
			+ 'static,
		T::Response: IntoResponse,
		T::Future: Send + 'static,
	{
		self.router(|router| router.fallback_service(service));
		self
	}

	fn method_not_allowed_fallback<H, T>(&mut self, handler: H) -> &mut Self
	where
		H: Handler<T, Self::State>,
		T: 'static,
	{
		self.router(|router| router.method_not_allowed_fallback(handler));
		self
	}
}

/// A no-op plugin that does nothing, used for testing purposes.
#[derive(Clone)]
pub struct EmptyPlugin;
impl Plugin for EmptyPlugin {
	fn build(&self, _: &mut App) {}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use axum::extract::Path;
	use axum::response::IntoResponse;
	use bevy::prelude::*;
	use sweet::prelude::*;


	#[derive(Resource)]
	struct MyResource(u32);



	fn static_handler() -> AppResult<impl IntoResponse> { Ok("Hello, World!") }
	fn returns_string_handler(
		path: In<Path<u32>>,
	) -> AppResult<impl IntoResponse> {
		Ok(path.to_string())
	}

	fn pipe1(_path: In<Path<u32>>, _: Query<()>) -> String { "hi".to_string() }

	fn pipe2(inner: In<String>, res: Res<MyResource>) -> String {
		format!("{} {}", *inner, res.0)
	}

	#[sweet::test]
	async fn works() {
		let mut router = App::new()
			.add_route("/", static_handler, EmptyPlugin)
			.take_router();
		router
			.oneshot_str("/")
			.await
			.unwrap()
			.xpect()
			.to_be("Hello, World!");
	}
	#[sweet::test]
	async fn returns_string() {
		let mut router = App::new()
			.add_route("/{id}", returns_string_handler, EmptyPlugin)
			.take_router();
		router.oneshot_str("/3").await.unwrap().xpect().to_be("3");
	}

	#[sweet::test]
	async fn piped_systems() {
		let mut router = App::new()
			.add_route("/{id}", pipe1.pipe(pipe2), |app: &mut App| {
				app.insert_resource(MyResource(42));
			})
			.take_router();
		router
			.oneshot_str("/3")
			.await
			.unwrap()
			.xpect()
			.to_be("hi 42");
	}
}
