use beet_core::prelude::*;
use beet_utils::utils::PipelineTarget;
use bevy::prelude::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// An asynchronous route handler, accepting and returning a [`World`].
#[derive(Clone, Component)]
pub struct RouteHandler(Arc<RouteHandlerFunc>);

type RouteHandlerFunc = dyn 'static
	+ Send
	+ Sync
	+ Fn(World, Entity) -> Pin<Box<dyn Future<Output = World> + Send>>;


pub fn no_request_err<T>() -> HttpError {
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
	/// handlers are infallible, any error is inserted into [`RouteHandlerOutput`]
	pub async fn run(&self, world: World, entity: Entity) -> World {
		(self.0)(world, entity).await
	}

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

	/// A route handler that passively runs a system,
	/// without expecting any system input or output.
	pub fn layer<T, Marker>(handler: T) -> Self
	where
		T: 'static + Send + Sync + Clone + IntoSystem<(), (), Marker>,
	{
		RouteHandler(Arc::new(move |mut world: World, _| {
			match world.run_system_cached(handler.clone()) {
				Ok(_) => {}
				Err(err) => {
					world.insert_resource(HttpError::from(err).into_response());
				}
			}
			Box::pin(async move { world })
		}))
	}

	/// An async route handler with output inserted as a [`Response`].
	/// This handler must return a tuple of [`(World, Out)`]
	pub fn new_async<Handler, In, InErr, Fut, Out>(handler: Handler) -> Self
	where
		In: TryFrom<Request, Error = InErr>,
		InErr: IntoResponse,
		Handler:
			'static + Send + Sync + Clone + FnOnce(World, In, Entity) -> Fut,
		Fut: 'static + Send + Future<Output = (World, Out)>,
		Out: 'static + Send + Sync + IntoResponse,
	{
		Self::layer_async(move |mut world, entity| {
			let func = handler.clone();
			async move {
				let Ok(input) = world
					.remove_resource::<Request>()
					.ok_or_else(|| no_request_err::<Handler>())
					.map_err(|err| {
						world.insert_resource(err.into_response());
						Err::<(), ()>(())
					})
				else {
					return world;
				};
				let Ok(input) = input.try_into().map_err(|err: InErr| {
					world.insert_resource(err.into_response());
					Err::<(), ()>(())
				}) else {
					return world;
				};

				let (mut world, out) = func(world, input, entity).await;
				world.insert_resource(out.into_response());
				world
			}
		})
	}


	/// An async route handler with output inserted as a [`Response`]
	pub fn layer_async<Handler, Fut>(handler: Handler) -> Self
	where
		Handler: 'static + Send + Sync + Clone + FnOnce(World, Entity) -> Fut,
		Fut: 'static + Send + Future<Output = World>,
	{
		RouteHandler(Arc::new(move |world, entity| {
			let func = handler.clone();
			Box::pin(async move { func(world, entity).await })
		}))
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn not_found() {
		Router::new_bundle(|| RouteHandler::new(HttpMethod::Get, || "howdy"))
			.oneshot("/foobar")
			.await
			.xpect()
			.to_be(Response::not_found());
	}
	#[sweet::test]
	async fn works() {
		Router::new_bundle(|| RouteHandler::new(HttpMethod::Get, || "howdy"))
			.oneshot("/")
			.await
			.status()
			.xpect()
			.to_be(StatusCode::OK);
	}
	#[sweet::test]
	async fn body() {
		Router::new_bundle(|| RouteHandler::new(HttpMethod::Get, || "hello"))
			.oneshot_str("/")
			.await
			.unwrap()
			.xpect()
			.to_be("hello");
	}

	#[sweet::test]
	async fn layers() {
		Router::new_bundle(|| {
			children![
				RouteHandler::layer(|mut req: ResMut<Request>| {
					req.set_body("jimmy");
				}),
				RouteHandler::new(HttpMethod::Get, |req: In<Request>| {
					let body = req.body_str().unwrap_or_default();
					format!("hello {}", body)
				}),
			]
		})
		.oneshot_str("/")
		.await
		.unwrap()
		.xpect()
		.to_be_str("hello jimmy");
	}
}
