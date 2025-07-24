use beet_core::prelude::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use serde::de::DeserializeOwned;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;


/// Marker type indicating this entity was spawned via [`RouteHandler::new_bundle`].
#[derive(Component)]
pub struct HandlerBundle;


/// A route layer that will always return the same html file for a given path,
/// making it suitable for ssg. Static routes can still have
/// client and server islands, but these are loaded from the html file
/// instead of being rendered on each request.
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct StaticRoute;

/// A boxed bundle, used to store a bundle in a [`RouteHandlerOutput`]
/// for later processing by layers.
pub struct BoxedBundle(
	pub Box<dyn 'static + Send + Sync + FnOnce(&mut World) -> Entity>,
);


impl BoxedBundle {
	pub fn new(bundle: impl Bundle) -> Self {
		Self(Box::new(move |world| world.spawn(bundle).id()))
	}
	pub fn add_to_world(self, world: &mut World) -> Entity { (self.0)(world) }
}
/// An asynchronous route handler, accepting and returning a [`World`].
#[derive(Clone, Component)]
pub struct RouteHandler(Arc<RouteHandlerFunc>);

type RouteHandlerFunc = dyn 'static
	+ Send
	+ Sync
	+ Fn(World) -> Pin<Box<dyn Future<Output = World> + Send>>;


impl RouteHandler {
	/// A route handler with output inserted as a [`Response`]
	pub fn new<T, Out, Marker>(handler: T) -> Self
	where
		T: 'static + Send + Sync + Clone + IntoSystem<(), Out, Marker>,
		Out: 'static + Send + Sync + IntoResponse,
	{
		Self::new_layer(move |world: &mut World| {
			let result = world.run_system_once(handler.clone());
			world.insert_resource(result.into_response());
		})
	}

	/// A route handler returning a bundle, which is inserted into the world
	/// with a [`HandlerBundle`] component.
	pub fn new_bundle<T, Out, Marker>(handler: T) -> Self
	where
		T: 'static + Send + Sync + Clone + IntoSystem<(), Out, Marker>,
		Out: 'static + Send + Sync + Bundle,
	{
		Self::new_layer(move |world: &mut World| {
			match world.run_system_once(handler.clone()) {
				Ok(out) => {
					world.spawn((HandlerBundle, out));
				}
				Err(err) => {
					world.insert_resource(err.into_response());
				}
			}
		})
	}

	/// A route handler accepting an input type to be extracted from the request.
	/// - For requests with no body, ie `GET`, the input is deserialized from the query parameters.
	/// - For requests with a body, ie `POST`, `PUT`, etc, the input is deserialized from the body.
	pub fn new_action<T, In, Out, Marker>(
		method: HttpMethod,
		handler: T,
	) -> Self
	where
		T: 'static + Send + Sync + Clone + IntoSystem<In, Out, Marker>,
		In: 'static + SystemInput,
		for<'a> In::Inner<'a>: DeserializeOwned,
		Out: 'static + Send + Sync + IntoResponse,
	{
		Self::new(move |world: &mut World| -> Result<Out> {
			let input = action_input::<In::Inner<'_>>(world, method)?;
			let out = world.run_system_cached_with(handler.clone(), input)?;
			Ok(out)
		})
	}

	/// A route handler that passively runs a system, without expecting any output.
	pub fn new_layer<T, Marker>(handler: T) -> Self
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
	pub fn new_async_system<Handler, Fut, Out>(handler: Handler) -> Self
	where
		Handler: 'static + Send + Sync + Clone + Fn(&mut World) -> Fut,
		Fut: 'static + Send + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResponse,
	{
		Self::new_async_layer(move |mut world: World| {
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
		Self::new_async_layer(move |world: World| {
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
	pub fn new_async_bundle<Handler, Fut, Out>(handler: Handler) -> Self
	where
		Handler: 'static + Send + Sync + Clone + Fn(World) -> Fut,
		Fut: 'static + Send + Future<Output = (World, Out)>,
		Out: 'static + Send + Sync + Bundle,
	{
		Self::new_async_layer(move |world: World| {
			let func = handler.clone();
			async move {
				let (mut world, out) = func(world).await;
				world.spawn((HandlerBundle, out));
				world
			}
		})
	}

	/// An async route handler with output inserted as a [`Response`]
	pub fn new_async_layer<Handler, Fut>(handler: Handler) -> Self
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


fn action_input<T: DeserializeOwned>(
	world: &mut World,
	method: HttpMethod,
) -> Result<T> {
	let request = world
		.remove_resource::<Request>()
		.ok_or_else(|| bevyhow!("no request found in world"))?;

	let input = match method.has_body() {
		true => {
			let json: Json<T> = request.try_into()?;
			json.0
		}
		false => {
			let query: QueryParams<T> = request.try_into()?;
			query.0
		}
	};
	Ok(input)
}


#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn not_found() {
		let mut world = World::new();
		Router::oneshot(&mut world, "/")
			.await
			.xpect()
			.to_be(Response::not_found());
	}
	#[sweet::test]
	async fn works() {
		let mut world = World::new();
		world.spawn(RouteHandler::new(|| "howdy"));
		Router::oneshot(&mut world, "/")
			.await
			.status()
			.xpect()
			.to_be(StatusCode::OK);
	}
	#[sweet::test]
	async fn body() {
		let mut world = World::new();
		world.spawn(RouteHandler::new(|| "hello"));
		Router::oneshot_str(&mut world, "/")
			.await
			.unwrap()
			.xpect()
			.to_be("hello");
	}

	#[sweet::test]
	async fn layers() {
		let mut world = World::new();
		world.spawn(children![
			RouteHandler::new_layer(|mut req: ResMut<Request>| {
				req.set_body("jimmy");
			}),
			RouteHandler::new(|req: Res<Request>| {
				let body = req.body_str().unwrap_or_default();
				format!("hello {}", body)
			})
		]);

		Router::oneshot_str(&mut world, "/")
			.await
			.unwrap()
			.xpect()
			.to_be_str("hello jimmy");
	}
}
