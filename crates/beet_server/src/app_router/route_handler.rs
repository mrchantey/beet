use crate::prelude::AppResult;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;




type RouteHandlerFunc =
	dyn 'static + Send + Sync + Fn(&mut World) -> AppResult<()>;

type AsyncRouteHandlerFunc = dyn 'static
	+ Send
	+ Sync
	+ for<'w> Fn(
		&'w mut World,
	) -> Pin<Box<dyn Future<Output = AppResult<()>> + Send + 'w>>;

/// The returned value from a [`RouteHandler`] or [`AsyncRouteHandler`] will be placed in this resource,
/// including `Result` and `()` types.
/// This will be used either for further processing by layers or converting to a [`Response`]
/// if it is a supported type.
#[derive(Resource, Deref)]
pub struct RouteHandlerOutput<T>(pub T);


/// A synchronous route handler, for async route handlers use [`AsyncRouteHandler`].
#[derive(Clone, Component)]
pub struct RouteHandler(Arc<RouteHandlerFunc>);

/// An asynchronous route handler, for bevy system handlers use [`RouteHandler`].
// We need this to differentiate from IntoSystem, because fn(&mut World) ->impl Future is a valid system
#[derive(Clone, Component)]
pub struct AsyncRouteHandler(Arc<AsyncRouteHandlerFunc>);




/// Each route may have any serializable metadata data associated with it,
/// to be loaded into the world before the route is called each time.
#[derive(Default, Clone, Component, Deref, Reflect)]
#[reflect(Default, Component)]
pub struct RouteScene {
	pub ron: String,
}




impl RouteHandler {
	/// Create a new sync route handler from a system
	pub fn new<T, Out, Marker>(system: T) -> Self
	where
		T: 'static + Send + Sync + Clone + IntoSystem<(), Out, Marker>,
		Out: 'static + Send + Sync,
	{
		RouteHandler(Arc::new(move |world: &mut World| {
			let out = world.run_system_once(system.clone())?;
			world.insert_resource(RouteHandlerOutput(out));
			Ok(())
		}))
	}

	/// Run the handler
	pub fn run(&self, world: &mut World) -> AppResult<()> { (self.0)(world) }
}

impl AsyncRouteHandler {
	/// Create a new async route handler
	pub fn new<T, Fut, Out>(func: T) -> Self
	where
		T: 'static + Send + Sync + Clone + for<'w> Fn(&'w mut World) -> Fut,
		Fut: 'static + Send + Future<Output = Out>,
		Out: 'static + Send + Sync,
	{
		AsyncRouteHandler(Arc::new(move |world: &mut World| {
			let func = func.clone();
			Box::pin(async move {
				let fut = func(world);
				let out = fut.await;
				world.insert_resource(RouteHandlerOutput(out));
				Ok(())
			})
		}))
	}

	/// Run the async handler
	pub async fn run(&self, world: &mut World) -> AppResult<()> {
		(self.0)(world).await
	}
}



#[cfg(test)]
mod test {
	use super::*;

	#[sweet::test]
	async fn works() {
		let _exclusive_system =
			RouteHandler::new(|_world: &mut World| -> Result<(), ()> {
				Ok(())
			});

		let _async_func =
			AsyncRouteHandler::new(|_world: &mut World| async move { 42u32 });
	}
}
