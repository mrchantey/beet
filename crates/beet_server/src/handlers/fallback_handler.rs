use crate::prelude::*;
use beet_rsx::as_beet::*;
use bevy::prelude::*;



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
		RouteHandler::async_layer(move |mut world: World| {
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
				RouteHandler::async_system(|_| async { "endpoint" }),
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
