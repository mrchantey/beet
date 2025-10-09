use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use std::future::Future;
use std::sync::Arc;



/// An asynchronous route handler, accepting and returning a [`World`].
#[derive(Clone, Component)]
#[component(on_add=collect_route_segments)]
pub struct RouteHandler(Arc<RouteHandlerFunc>);

/// Insert a [`RouteSegments`] containing the [`PathFilter`] for this
/// handler and its ancestors.
fn collect_route_segments(mut world: DeferredWorld, cx: HookContext) {
	let entity = cx.entity;
	world
		.commands()
		.queue(move |world: &mut World| -> Result<()> {
			let segments =
				world.run_system_cached_with(RouteSegments::collect, entity)?;
			world.entity_mut(entity).insert(segments);
			Ok(())
		});
}


type RouteHandlerFunc =
	dyn 'static + Send + Sync + Fn(World, Entity) -> SendBoxedFuture<World>;


pub fn no_request_err<T>() -> HttpError {
	HttpError::internal_error(format!(
		"
Handler Error: {}\n\n
No request found in world. This can occur when two handlers compete
to remove the request resource, try adding an ExactPath component to endpoints
that consume the request
	",
		std::any::type_name::<T>()
	))
}

impl RouteHandler {
	/// handlers are infallible, any error is inserted into [`RouteHandlerOutput`]
	pub async fn run(&self, world: World, entity: Entity) -> World {
		(self.0)(world, entity).await
	}

	/// A route handler with output inserted as a [`Response`], these add
	/// an [`ExactPath`] component which means the path must not contain
	/// trailing segments to match this handler.
	pub fn endpoint<T, In, InM, Out, Marker>(handler: T) -> (Endpoint, Self)
	where
		T: 'static + Send + Sync + Clone + IntoSystem<In, Out, Marker>,
		Out: 'static + Send + Sync + IntoResponse,
		In: 'static + SystemInput,
		for<'a> In::Inner<'a>: FromRequest<InM>,
	{
		let handler = move |world: &mut World| -> Result<Out, Response> {
			let req = world
				.remove_resource::<Request>()
				.ok_or_else(|| no_request_err::<T>())?;
			let input = In::Inner::from_request_sync(req)?;
			let out = world
				.run_system_cached_with(handler.clone(), input)
				.map_err(|err| HttpError::from(err).into_response())?;
			Ok(out)
		};

		(
			Endpoint,
			Self::layer(move |world: &mut World| {
				let res = handler(world).into_response();
				world.insert_resource(res);
			}),
		)
	}

	/// Create a route handler that will simply return a 200 Ok response.
	pub fn ok() -> (Endpoint, Self) {
		fn noop() {}
		Self::endpoint(noop)
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
	pub fn new_async<Handler, In, InM, Fut, Out>(handler: Handler) -> Self
	where
		In: FromRequest<InM>,
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
				let Ok(input) = In::from_request_sync(input).map_err(|err| {
					world.insert_resource(err);
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
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;


	#[sweet::test]
	async fn not_found() {
		Router::new_bundle(|| RouteHandler::endpoint(|| "howdy"))
			.oneshot("/foobar")
			.await
			.xpect_eq(Response::not_found());
	}
	#[sweet::test]
	async fn works() {
		Router::new_bundle(|| RouteHandler::endpoint(|| "howdy"))
			.oneshot("/")
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}
	#[sweet::test]
	async fn body() {
		Router::new_bundle(|| RouteHandler::endpoint(|| "hello"))
			.oneshot_str("/")
			.await
			.unwrap()
			.xpect_eq("hello");
	}

	#[sweet::test]
	async fn layers() {
		Router::new_bundle(|| {
			children![
				RouteHandler::layer(|mut req: ResMut<Request>| {
					req.set_body("jimmy");
				}),
				RouteHandler::endpoint(|req: In<Request>| {
					let body = req.0.body.try_into_bytes().unwrap_or_default();
					let body = std::str::from_utf8(&body).unwrap_or_default();
					format!("hello {}", body)
				}),
			]
		})
		.oneshot_str("/")
		.await
		.unwrap()
		.xpect_str("hello jimmy");
	}
}
