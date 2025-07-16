use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::prelude::AppResult;


type RouteHandlerFunc = dyn 'static
	+ Send
	+ Sync
	+ Fn(World) -> Pin<Box<dyn Future<Output = World> + Send>>;

/// The returned value from a [`RouteHandler`] will be placed in this resource,
/// including [`Result`] and [`()`] types.
/// This will be used either for further processing by layers or converting to a [`Response`]
/// if it is a supported type.
#[derive(Resource, Deref)]
pub struct RouteHandlerOutput<T>(pub T);


/// A route layer that will always return the same html file for a given path,
/// making it suitable for ssg. Static routes can still have
/// client and server islands, but these are loaded from the html file
/// instead of being rendered on each request.
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct StaticRoute;



/// Each route may have any serializable metadata data associated with it,
/// to be loaded into the world before the route is called each time.
#[derive(Default, Clone, Component, Deref, Reflect)]
#[reflect(Default, Component)]
pub struct RouteScene {
	pub ron: String,
}

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
/// An asynchronous route handler
#[derive(Clone, Component)]
pub struct RouteHandler(Arc<RouteHandlerFunc>);


impl RouteHandler {
	/// Create a new sync route handler from a system
	pub fn new<T, Out, Marker>(handler: T) -> Self
	where
		T: 'static + Send + Sync + Clone + IntoSystem<(), Out, Marker>,
		Out: 'static + Send + Sync,
	{
		Self::new_mapped(handler, |out| out)
	}

	pub fn new_bundle<T, Out, Marker>(handler: T) -> Self
	where
		T: 'static + Send + Sync + Clone + IntoSystem<(), Out, Marker>,
		Out: 'static + Send + Sync + Bundle,
	{
		Self::new_mapped(handler, BoxedBundle::new)
	}

	/// Create a new handler from a system returning a bundle,
	/// placing the bundle in a [`BoxedBundle`] for convenient processing
	/// by layers like [`bundle_to_html`]
	pub fn new_mapped<T, Out, Out2, Marker>(
		handler: T,
		map: impl 'static + Send + Sync + Fn(Out) -> Out2,
	) -> Self
	where
		T: 'static + Send + Sync + Clone + IntoSystem<(), Out, Marker>,
		Out: 'static + Send + Sync,
		Out2: 'static + Send + Sync,
	{
		RouteHandler(Arc::new(move |mut world: World| {
			match world.run_system_once(handler.clone()) {
				Ok(out) => {
					world.insert_resource(RouteHandlerOutput(map(out)));
				}
				Err(run_system_err) => {
					// resemble the expected output as close as possible
					let result: AppResult<Out2> = Err(run_system_err.into());
					world.insert_resource(RouteHandlerOutput(result));
				}
			}
			Box::pin(async move { world })
		}))
	}
	/// Create a new async route handler from a system
	pub fn new_async<Handler, Fut, Out>(handler: Handler) -> Self
	where
		Handler: 'static + Send + Sync + Clone + Fn(World) -> Fut,
		Fut: 'static + Send + Future<Output = (World, Out)>,
		Out: 'static + Send + Sync,
	{
		Self::new_async_mapped(handler, |out| out)
	}
	pub fn new_async_bundle<Handler, Fut, Out>(handler: Handler) -> Self
	where
		Handler: 'static + Send + Sync + Clone + Fn(World) -> Fut,
		Fut: 'static + Send + Future<Output = (World, Out)>,
		Out: 'static + Send + Sync + Bundle,
	{
		Self::new_async_mapped(handler, BoxedBundle::new)
	}
	/// Create a new async route handler from a system
	pub fn new_async_mapped<Handler, Fut, Out, Out2>(
		handler: Handler,
		map: impl 'static + Send + Sync + Clone + Fn(Out) -> Out2,
	) -> Self
	where
		Handler: 'static + Send + Sync + Clone + Fn(World) -> Fut,
		// &mut World is so difficult to do
		Fut: 'static + Send + Future<Output = (World, Out)>,
		Out: 'static + Send + Sync,
		Out2: 'static + Send + Sync,
	{
		RouteHandler(Arc::new(move |world: World| {
			let func = handler.clone();
			let map = map.clone();
			Box::pin(async move {
				let (mut world, out) = func(world).await;
				world.insert_resource(RouteHandlerOutput(map(out)));
				world
			})
		}))
	}


	/// handlers are infallible, any error is inserted into [`RouteHandlerOutput`]
	pub async fn run(&self, world: World) -> World { (self.0)(world).await }
}



#[cfg(test)]
mod test {
	use super::*;

	#[sweet::test]
	async fn works() {
		let mut world = World::new();
		world = RouteHandler::new_bundle(|| ()).run(world).await;
		world.resource::<RouteHandlerOutput<BoxedBundle>>();

		async fn foo(world: World) -> (World, ()) { (world, ()) }

		let mut world = World::new();
		world = RouteHandler::new_async_bundle(foo).run(world).await;
		world.resource::<RouteHandlerOutput<BoxedBundle>>();
	}
}
