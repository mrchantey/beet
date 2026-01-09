use crate::prelude::*;
use beet_core::prelude::*;


/// Trait for handling oneshot requests on async types (immutable self)
pub trait OneshotRequest {
	/// Handle a single request and return the response
	fn oneshot(
		&self,
		req: impl Into<Request>,
	) -> impl Future<Output = Response>;

	/// Handle a single request bundle and return the response
	fn oneshot_bundle(
		&self,
		bundle: impl Bundle,
	) -> impl Future<Output = Response>;

	/// Convenience method for testing, unwraps a 200 response string
	fn oneshot_str(
		&self,
		req: impl Into<Request>,
	) -> impl Future<Output = String>;
}

/// Trait for handling oneshot requests on mutable world types
pub trait OneshotRequestMut {
	/// Handle a single request and return the response
	fn oneshot(
		&mut self,
		req: impl Into<Request>,
	) -> impl Future<Output = Response>;

	/// Handle a single request bundle and return the response
	fn oneshot_bundle(
		&mut self,
		bundle: impl Bundle,
	) -> impl Future<Output = Response>;

	/// Convenience method for testing, unwraps a 200 response string
	fn oneshot_str(
		&mut self,
		req: impl Into<Request>,
	) -> impl Future<Output = String>;
}


impl OneshotRequest for AsyncEntity {
	fn oneshot(
		&self,
		req: impl Into<Request>,
	) -> impl Future<Output = Response> {
		async move { self.oneshot_bundle(req.into()).await }
	}

	fn oneshot_bundle(
		&self,
		req: impl Bundle,
	) -> impl Future<Output = Response> {
		async move { handle_request(self.clone(), req).await }
	}

	fn oneshot_str(
		&self,
		req: impl Into<Request>,
	) -> impl Future<Output = String> {
		let req = req.into();
		async move {
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


impl OneshotRequestMut for EntityWorldMut<'_> {
	fn oneshot(
		&mut self,
		req: impl Into<Request>,
	) -> impl Future<Output = Response> {
		self.oneshot_bundle(req.into())
	}

	fn oneshot_bundle(
		&mut self,
		bundle: impl Bundle,
	) -> impl Future<Output = Response> {
		self.run_async_then(async move |entity| {
			entity.oneshot_bundle(bundle).await
		})
	}

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


impl OneshotRequestMut for World {
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

	fn oneshot_bundle(
		&mut self,
		bundle: impl Bundle,
	) -> impl Future<Output = Response> {
		let entity = self
			.query_filtered::<Entity, With<ExchangeSpawner>>()
			.single(self)
			.expect("Expected a single ExchangeSpawner");
		async move { self.entity_mut(entity).oneshot_bundle(bundle).await }
	}

	fn oneshot_str(
		&mut self,
		req: impl Into<Request>,
	) -> impl Future<Output = String> {
		let req = req.into();
		let entity = self
			.query_filtered::<Entity, With<ExchangeSpawner>>()
			.single(self)
			.expect("Expected a single ExchangeSpawner");
		async move { self.entity_mut(entity).oneshot_str(req).await }
	}
}


impl OneshotRequest for AsyncWorld {
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

	fn oneshot_bundle(
		&self,
		bundle: impl Bundle,
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
			self.entity(server).oneshot_bundle(bundle).await
		}
	}

	fn oneshot_str(
		&self,
		req: impl Into<Request>,
	) -> impl Future<Output = String> {
		async move {
			let server = self
				.with_then(|world| {
					world
						.query_filtered::<Entity, With<ExchangeSpawner>>()
						.single(world)
						.expect("Expected a single ExchangeSpawner")
				})
				.await;
			self.entity(server).oneshot_str(req).await
		}
	}
}


/// - Creates a child of the server inserting the [`Request`] component
/// - Adds a one-shot observer for [`On<Insert, Response>`],
///   then takes the response and despawns the entity.
/// the default handler adds about 100us to a request that
/// doesnt involve mutating the world or running systems: (40us vs 140us)
/// ## Panics
///
/// Panics if the provided server entity has no ExchangeHandler
async fn handle_request(server: AsyncEntity, request: impl Bundle) -> Response {
	let server_id = server.id();
	let (send, recv) = async_channel::bounded(1);
	let exchange_entity = server
		.world()
		.with_then(move |world| {
			let spawner = world
				.entity_mut(server_id)
				.get::<ExchangeSpawner>()
				.cloned()
				.expect("Server has no ExchangeHandler");

			let agent = spawner.spawn(world);
			world
				.entity_mut(agent)
				// add observer before inserting request to handle immediate response
				.observe(
					move |ev: On<Insert, Response>, mut commands: Commands| {
						let exchange = ev.event_target();
						let send = send.clone();
						commands.queue(move |world: &mut World| -> Result {
							let response = world
								.entity_mut(exchange)
								.take::<Response>()
								.ok_or_else(|| {
									bevyhow!(
										"Response inserted but missing from exchange"
									)
								})?;
							send.try_send(response)?;
							Ok(())
						});
					},
				)
				.insert((request, ExchangeOf(server_id)))
				.id()
		})
		.await;

	let response = recv.recv().await.unwrap_or_else(|_| {
		error!("Sender was dropped, was the world dropped?");
		Response::from_status(StatusCode::INTERNAL_SERVER_ERROR)
	});

	// cleanup exchange entity after response is received
	server
		.world()
		.with_then(move |world| {
			if let Ok(exchange) = world.get_entity_mut(exchange_entity) {
				exchange.despawn();
			}
		})
		.await;

	response
}
