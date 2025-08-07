use crate::prelude::*;
use beet_rsx::as_beet::*;
// use bevy::ecs::system::BoxedSystem;
use bevy::prelude::*;

/// Add this to a [`RouteHandler`] to ensure it only runs if there is no [`Response`] resource
// #[derive(Component)]
// pub struct RunIf(Box<dyn Fn(&mut World) -> bool + Send + Sync>);

// impl RunIf {
// 	pub fn no_response() -> Self {
// 		Self(Box::new(|world: &mut World| {
// 			!world.contains_resource::<Response>()
// 		}))
// 	}
// 	pub fn should_run(&self, world: &mut World) -> bool { (self.0)(world) }
// }


impl RouteHandler {
	// pub fn fallback<M>(handler: impl EndpointSystem<M>) -> impl Bundle{

	// 	RouteHandler::new

	// }


	/// An async [`RouteHandler`] that will only run if there is no [`Response`] resource in the world.
	pub fn fallback_async<Handler, Fut, Out>(handler: Handler) -> RouteHandler
	where
		for<'a> Handler:
			'static + Send + Sync + Clone + Fn(&'a mut World) -> Fut,
		for<'a> Fut: 'a + Send + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResponse,
	{
		RouteHandler::layer_async(move |mut world: World| {
			let handler = handler.clone();
			async move {
				if !world.contains_resource::<Response>() {
					let response = handler(&mut world).await.into_response();
					world.insert_resource(response);
				}
				world
			}
		})
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[rustfmt::skip]
	#[sweet::test]
	async fn runs_async() {
		Router::new(|app: &mut App| {
			app.world_mut().spawn(children![
				RouteHandler::fallback_async(|_| async { "fallback" })
			]);
		})
		.oneshot_str("/")
		.await
		.unwrap()
		.xpect()
		.to_be_str("fallback");
	}
	#[rustfmt::skip]
	#[sweet::test]
	async fn skips_async() {
		Router::new(|app: &mut App| {
			app.world_mut().spawn(children![
				RouteHandler::new_async(|world| async { (world,"endpoint") }),
				RouteHandler::fallback_async(|_| async { "fallback" })
			]);
		})
		.oneshot_str("/")
		.await
		.unwrap()
		.xpect()
		.to_be_str("endpoint");
	}
}
