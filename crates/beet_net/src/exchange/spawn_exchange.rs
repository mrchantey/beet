//! Simple exchange pattern for request/response handling.
//!
//! This module provides [`spawn_exchange`], which creates an exchange handler
//! that spawns a new entity for each request, observes the response insertion,
//! and completes the exchange.

use crate::prelude::*;
use beet_core::prelude::*;

/// Creates an exchange handler that spawns a new entity for each request.
///
/// The provided function is called for each incoming request to create the bundle
/// that will handle the exchange. The handler must insert a [`Response`] component
/// on the spawned entity to complete the exchange.
///
/// ## Execution Flow
///
/// 1. [`ExchangeStart`] is triggered on the spawner entity
/// 2. A new entity is spawned with the provided bundle
/// 3. [`Request`] is inserted on the entity (after the bundle, allowing observers to be ready)
/// 4. When [`Response`] is inserted, it is taken and sent via the exchange channel
///
/// ## Example
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// let mut world = World::new();
/// let mut entity = world.spawn(spawn_exchange(|| {
///     OnSpawn::observe(
///         |ev: On<Insert, Request>,
///          mut commands: Commands,
///          requests: Query<&Request>| {
///             // Mirror the request back as the response
///             commands.entity(ev.event_target()).insert(
///                 requests.get(ev.event_target()).unwrap().mirror_parts(),
///             );
///         },
///     )
/// }));
/// ```
pub fn spawn_exchange(func: impl BundleFunc) -> impl Bundle {
	OnSpawn::observe(
		move |ev: On<ExchangeStart>, mut commands: Commands| -> Result {
			let server_entity = ev.event_target();
			let (req, cx) = ev.take()?;
			let mut entity = commands.spawn((
				ChildOf(server_entity),
				OnSpawn::observe(end_on_insert_response),
				func.clone().bundle_func(),
				cx,
			));
			// insert request after spawner, giving it a
			// chance to insert observers
			entity.insert(req);

			Ok(())
		},
	)
}

/// End the exchange when a Response is inserted
// this would be an exclusive observer but thats not yet supported
fn end_on_insert_response(
	ev: On<Insert, Response>,
	mut commands: Commands,
) -> Result {
	let exchange_entity = ev.event_target();
	commands
		.entity(exchange_entity)
		.queue(take_and_send_response);
	Ok(())
}

fn take_and_send_response(mut entity: EntityWorldMut) -> Result {
	let response = entity
		.take::<Response>()
		.unwrap_or_else(|| Response::not_found());
	entity
		.get::<ExchangeContext>()
		.ok_or_else(|| bevyhow!("ExchangeEnd not found"))?
		.clone()
		.end(entity, response)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn works() {
		World::new()
			.spawn(spawn_exchange(|| {
				OnSpawn::observe(
					|ev: On<Insert, Request>,
					 mut commands: Commands,
					 requests: Query<&Request>| {
						commands.entity(ev.event_target()).insert(
							requests
								.get(ev.event_target())
								.unwrap()
								.mirror_parts(),
						);
					},
				)
			}))
			.exchange(Request::get("/foo"))
			.await
			.path_string()
			.xpect_eq("/foo");
	}
}
