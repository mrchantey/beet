use crate::prelude::*;
use beet_core::prelude::*;
use std::sync::Arc;


/// The function called for each request,
/// see [`default_handler`] for the default implementation.
#[derive(Clone, Deref, Component)]
pub struct ExchangeSpawner {
	func: SpawnFunc,
}

/// We accept an &mut World to allow for entity pooling
type SpawnFunc = Arc<Box<dyn 'static + Send + Sync + Fn(&mut World) -> Entity>>;

impl ExchangeSpawner {
	pub fn new<Func>(func: Func) -> Self
	where
		Func: 'static + Send + Sync + Fn(&mut World) -> Entity,
	{
		Self {
			func: Arc::new(Box::new(func)),
		}
	}
	pub fn new_bundle<F, B>(func: F) -> Self
	where
		F: 'static + Send + Sync + Fn() -> B,
		B: Bundle,
	{
		Self::new(move |world: &mut World| world.spawn(func()).id())
	}

	pub fn new_handler(
		func: impl 'static
		+ Send
		+ Sync
		+ Clone
		+ FnOnce(EntityWorldMut, Request) -> Response,
	) -> Self {
		Self::new_bundle(move || {
			let func = func.clone();
			OnSpawn::observe(
				move |ev: On<Insert, Request>, mut commands: Commands| {
					let func = func.clone();
					let entity = ev.event_target();
					commands.queue(move |world: &mut World| -> Result {
						let req = world
							.entity_mut(entity)
							.take::<Request>()
							.ok_or_else(|| {
								bevyhow!(
									"Exchange entity missing Request component"
								)
							})?;
						let response = func(world.entity_mut(entity), req);
						world.entity_mut(entity).insert(response);
						Ok(())
					});
				},
			)
		})
	}

	pub fn new_handler_async<Fut>(
		func: impl 'static
		+ Send
		+ Sync
		+ Clone
		+ FnOnce(AsyncEntity, Request) -> Fut,
	) -> Self
	where
		Fut: Send + Future<Output = Response>,
	{
		Self::new_bundle(move || {
			let func = func.clone();
			OnSpawn::observe(
				move |ev: On<Insert, Request>, mut commands: AsyncCommands| {
					let func = func.clone();
					let entity = ev.event_target();
					commands.run(async move |world: AsyncWorld| -> Result {
						let entity = world.entity(entity);
						let req = world
							.entity(entity.id())
							.take::<Request>()
							.await
							.ok_or_else(|| {
								bevyhow!(
									"Exchange entity missing Request component"
								)
							})?;
						let response = func(entity.clone(), req).await;
						entity.insert(response);
						Ok(())
					});
				},
			)
		})
	}
	fn spawn(&self, world: &mut World) -> Entity { (self.func)(world) }

	/// - Creates a child of the server inserting the [`Request`] component
	/// - Adds a one-shot observer for [`On<Insert, Response>`],
	///   then takes the response and despawns the entity.
	/// the default handler adds about 100us to a request that
	/// doesnt involve mutating the world or running systems: (40us vs 140us)
	/// ## Panics
	///
	/// Panics if the provided server entity has no ExchangeHandler
	pub async fn handle_request(
		server: AsyncEntity,
		request: Request,
	) -> Response {
		let server_id = server.id();
		let (send, recv) = async_channel::bounded(1);
		let exchange_entity = server
			.world()
			.with_then(move |world| {
				let exchange_handler = world
					.entity_mut(server_id)
					.get::<ExchangeSpawner>()
					.cloned()
					.expect("Server has no ExchangeHandler");

				let entity = exchange_handler.spawn(world);
				world
					.entity_mut(entity)
					// add observer before inserting request to handle immediate response
					.observe(
						move |ev: On<Insert, Response>,
						      mut commands: Commands| {
							let exchange = ev.event_target();
							let send = send.clone();
							commands.queue(
								move |world: &mut World| -> Result {
									let response = world
										.entity_mut(exchange)
										.take::<Response>()
										.ok_or_else(|| {
											bevyhow!(
												"Exchange entity missing Response component"
											)
										})?;
									send.try_send(response)?;
									Ok(())
								},
							);
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

	pub fn mirror() -> Self {
		Self::new_handler(|_, req| {
			Response::new(
				ResponseParts {
					parts: req.parts().parts().clone(),
					status: StatusCode::OK,
				},
				req.body,
			)
		})
	}
}


impl Default for ExchangeSpawner {
	fn default() -> Self { Self::mirror() }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let store = Store::default();
		app.world_mut().spawn(ExchangeSpawner::default()).run_async(
			async move |entity| {
				let res = ExchangeSpawner::handle_request(
					entity.clone(),
					Request::get("foo"),
				)
				.await;
				store.set(Some(res.status()));
				entity.world().write_message(AppExit::Success);
			},
		);
		app.run();
		store.get().unwrap().xpect_eq(StatusCode::OK);
	}
}
