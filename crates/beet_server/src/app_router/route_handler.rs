use beet_core::prelude::*;
use beet_utils::utils::PipelineTarget;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use serde::de::DeserializeOwned;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;


/// Marker type indicating this entity was spawned via [`RouteHandler::new_bundle`].
#[derive(Component)]
pub struct HandlerBundle;



/// An asynchronous route handler, accepting and returning a [`World`].
#[derive(Clone, Component)]
pub struct RouteHandler(Arc<RouteHandlerFunc>);

type RouteHandlerFunc = dyn 'static
	+ Send
	+ Sync
	+ Fn(World) -> Pin<Box<dyn Future<Output = World> + Send>>;


fn no_request_err<T>() -> HttpError {
	HttpError::internal_error(format!(
		"
Handler Error: {}\n\n
No request found in world. This can occur when two handlers compete
to remove the request resource, try explicitly adding an Endpoint to each
	
	",
		std::any::type_name::<T>()
	))
}

impl RouteHandler {
	/// A route handler with output inserted as a [`Response`]
	pub fn new<T, In, InErr, Out, Marker>(
		endpoint: impl Into<Endpoint>,
		handler: T,
	) -> (Endpoint, Self)
	where
		T: 'static + Send + Sync + Clone + IntoSystem<In, Out, Marker>,
		Out: 'static + Send + Sync + IntoResponse,
		In: 'static + SystemInput,
		for<'a> In::Inner<'a>: TryFrom<Request, Error = InErr>,
		InErr: IntoResponse,
	{
		let handler = move |world: &mut World| -> Result<Out, Response> {
			let input = world
				.remove_resource::<Request>()
				.ok_or_else(|| no_request_err::<T>())?
				.try_into()
				.map_err(|err: InErr| err.into_response())?;
			let out = world
				.run_system_cached_with(handler.clone(), input)
				.map_err(|err| HttpError::from(err).into_response())?;
			Ok(out)
		};

		Self::layer(move |world: &mut World| {
			let res = handler(world).into_response();
			world.insert_resource(res);
		})
		.xmap(move |handler| (endpoint.into(), handler))
	}

	/// A route handler returning a bundle, which is inserted into the world
	/// with a [`HandlerBundle`] component.
	pub fn bundle<T, In, InErr, Out, Marker>(
		endpoint: impl Into<Endpoint>,
		handler: T,
	) -> (Endpoint, Self)
	where
		T: 'static + Send + Sync + Clone + IntoSystem<In, Out, Marker>,
		In: 'static + SystemInput,
		for<'a> In::Inner<'a>: TryFrom<Request, Error = InErr>,
		InErr: IntoResponse,
		Out: 'static + Send + Sync + Bundle,
	{
		let handler = move |world: &mut World| -> Result<(), Response> {
			let input = world
				.remove_resource::<Request>()
				.ok_or_else(|| no_request_err::<T>())?
				.try_into()
				.map_err(|err: InErr| err.into_response())?;
			match world.run_system_once_with(handler.clone(), input) {
				Ok(out) => {
					world.spawn((HandlerBundle, out));
				}
				Err(err) => {
					world.insert_resource(err.into_response());
				}
			}
			Ok(())
		};

		Self::layer(move |world: &mut World| {
			if let Err(err) = handler(world) {
				world.insert_resource(err);
			}
		})
		.xmap(move |handler| (endpoint.into(), handler))
	}

	/// A route handler accepting an input type to be extracted from the request.
	/// - For requests with no body, ie `GET`, the input is deserialized from the query parameters.
	/// - For requests with a body, ie `POST`, `PUT`, etc, the input is deserialized from the body.
	pub fn action<T, Input, Out, Marker>(
		endpoint: impl Into<Endpoint>,
		handler: T,
	) -> (Endpoint, Self)
	where
		T: 'static + Send + Sync + Clone + IntoSystem<Input, Out, Marker>,
		Input: 'static + SystemInput,
		for<'a> Input::Inner<'a>: DeserializeOwned,
		Out: 'static + Send + Sync + IntoResponse,
	{
		let endpoint = endpoint.into();
		match endpoint.method().has_body() {
			// ie `POST`, `PUT`, etc
			true => Self::new(
				endpoint,
				move |val: In<Json<Input::Inner<'_>>>,
				      world: &mut World|
				      -> Result<Out> {
					let out = world
						.run_system_cached_with(handler.clone(), val.0.0)?;
					Ok(out)
				},
			),
			// ie `GET`, `DELETE`, etc
			false => Self::new(
				endpoint,
				move |val: In<JsonQueryParams<Input::Inner<'_>>>,
				      world: &mut World|
				      -> Result<Out> {
					let out = world
						.run_system_cached_with(handler.clone(), val.0.0)?;
					Ok(out)
				},
			),
		}
	}


	/// A route handler that passively runs a system, 
	/// without expecting any system input or output.
	pub fn layer<T, Marker>(handler: T) -> Self
	where
		T: 'static + Send + Sync + Clone + IntoSystem<(), (), Marker>,
	{
		RouteHandler(Arc::new(move |mut world: World| {
			match world.run_system_once(handler.clone()) {
				Ok(_) => {}
				Err(err) => {
					world.insert_resource(err.into_response());
				}
			}
			Box::pin(async move { world })
		}))
	}

	/// An async route handler with output inserted as a [`Response`]
	pub fn async_system<Handler, Fut, Out>(handler: Handler) -> Self
	where
		Handler: 'static + Send + Sync + Clone + Fn(&mut World) -> Fut,
		Fut: 'static + Send + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResponse,
	{
		Self::async_layer(move |mut world: World| {
			let func = handler.clone();
			async move {
				let out = func(&mut world).await;
				world.insert_resource(out.into_response());
				world
			}
		})
	}
	/// An async route handler with output inserted as a [`Response`]
	pub fn new_async<Handler, Fut, Out>(handler: Handler) -> Self
	where
		Handler: 'static + Send + Sync + Clone + Fn(World) -> Fut,
		Fut: 'static + Send + Future<Output = (World, Out)>,
		Out: 'static + Send + Sync + IntoResponse,
	{
		Self::async_layer(move |world: World| {
			let func = handler.clone();
			async move {
				let (mut world, out) = func(world).await;
				world.insert_resource(out.into_response());
				world
			}
		})
	}

	/// An async route handler returning a bundle, which is inserted into the world
	/// with a [`HandlerBundle`] component.
	pub fn async_bundle<Handler, Fut, Out>(handler: Handler) -> Self
	where
		Handler: 'static + Send + Sync + Clone + Fn(World) -> Fut,
		Fut: 'static + Send + Future<Output = (World, Out)>,
		Out: 'static + Send + Sync + Bundle,
	{
		Self::async_layer(move |world: World| {
			let func = handler.clone();
			async move {
				let (mut world, out) = func(world).await;
				world.spawn((HandlerBundle, out));
				world
			}
		})
	}

	/// An async route handler with output inserted as a [`Response`]
	pub fn async_layer<Handler, Fut>(handler: Handler) -> Self
	where
		Handler: 'static + Send + Sync + Clone + Fn(World) -> Fut,
		Fut: 'static + Send + Future<Output = World>,
	{
		RouteHandler(Arc::new(move |world: World| {
			let func = handler.clone();
			Box::pin(async move { func(world).await })
		}))
	}

	/// handlers are infallible, any error is inserted into [`RouteHandlerOutput`]
	pub async fn run(&self, world: World) -> World { (self.0)(world).await }
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn not_found() {
		Router::new(|app: &mut App| {
			app.world_mut()
				.spawn(RouteHandler::new(HttpMethod::Get, || "howdy"));
		})
		.oneshot("/foobar")
		.await
		.xpect()
		.to_be(Response::not_found());
	}
	#[sweet::test]
	async fn works() {
		Router::new(|app: &mut App| {
			app.world_mut()
				.spawn(RouteHandler::new(HttpMethod::Get, || "howdy"));
		})
		.oneshot("/")
		.await
		.status()
		.xpect()
		.to_be(StatusCode::OK);
	}
	#[sweet::test]
	async fn bundle() {
		fn foo(_bar: Query<Entity>) -> impl Bundle + use<> {
			rsx! {<div>hello</div>}
		}

		Router::new(|app: &mut App| {
			app.world_mut()
				.spawn(RouteHandler::bundle(HttpMethod::Get, foo));
		})
		.oneshot_str("/")
		.await
		.unwrap()
		.xpect()
		.to_be(
			"<!DOCTYPE html><html><head></head><body><div>hello</div></body></html>",
		);
	}
	#[sweet::test]
	async fn body() {
		Router::new(|app: &mut App| {
			app.world_mut()
				.spawn(RouteHandler::new(HttpMethod::Get, || "hello"));
		})
		.oneshot_str("/")
		.await
		.unwrap()
		.xpect()
		.to_be("hello");
	}

	#[sweet::test]
	async fn layers() {
		Router::new(|app: &mut App| {
			app.world_mut().spawn(children![
				RouteHandler::layer(|mut req: ResMut<Request>| {
					req.set_body("jimmy");
				}),
				RouteHandler::new(HttpMethod::Get, |req: In<Request>| {
					let body = req.body_str().unwrap_or_default();
					format!("hello {}", body)
				})
			]);
		})
		.oneshot_str("/")
		.await
		.unwrap()
		.xpect()
		.to_be_str("hello jimmy");
	}
}
