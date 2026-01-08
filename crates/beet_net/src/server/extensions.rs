use crate::prelude::*;
use beet_core::prelude::*;


#[extend::ext(name=AsyncEntityRouterExt)]
pub impl AsyncEntity {
	/// Handle a single request and return the response
	fn oneshot(
		&self,
		req: impl Into<Request>,
	) -> impl Future<Output = Response> {
		async move {
			ExchangeSpawner::handle_request(self.clone(), req.into()).await
		}
	}
}
#[extend::ext(name=WorldRouterExt)]
pub impl World {
	/// Handle a single request and return the response
	/// ## Panics
	/// Panics if there is not exactly one `Router` in the world.
	fn oneshot(
		&mut self,
		req: impl Into<Request>,
	) -> impl Future<Output = Response> {
		let req = req.into();
		let entity = self
			.query_filtered::<Entity, With<ExchangeSpawner>>()
			.single(self)
			.expect("Expected a single ExchangeSpawner");
		async move { self.entity_mut(entity).oneshot(req).await }
	}
}
#[extend::ext(name=EntityWorldMutRouterExt)]
pub impl EntityWorldMut<'_> {
	/// Handle a single request and return the response
	fn oneshot(
		&mut self,
		req: impl Into<Request>,
	) -> impl Future<Output = Response> {
		let req = req.into();
		self.oneshot_bundle(req)
	}
	/// Handle a single request bundle and return the response
	fn oneshot_bundle(
		&mut self,
		bundle: impl Bundle,
	) -> impl Future<Output = Response> {
		self.run_async_then(async move |entity| {
			ExchangeSpawner::handle_request(entity, bundle)
				.await
				.into_response()
		})
	}

	#[cfg(test)]
	/// Convenience method for testing, unwraps a 200 response string
	fn oneshot_str(
		&mut self,
		req: impl Into<Request>,
	) -> impl Future<Output = String> {
		let req = req.into();
		async {
			self.oneshot(req)
				.await
				.into_result()
				.await
				.unwrap()
				.text()
				.await
				.expect("Expected text body")
		}
	}
}



#[extend::ext(name=AsyncWorldRouterExt)]
pub impl AsyncWorld {
	/// Handle a single request and return the response
	/// ## Panics
	/// Panics if there is not exactly one `Router` in the world.
	fn oneshot(
		&self,
		req: impl Into<Request>,
	) -> impl Future<Output = Response> {
		async move {
			let server = self
				.with_then(|world| {
					world
						.query_filtered::<Entity, With<ExchangeSpawner>>()
						.single(world)
						.expect("Expected a single ExchangeSpawner")
				})
				.await;
			self.entity(server).oneshot(req).await
		}
	}
}
