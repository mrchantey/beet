//! Direct handler exchange patterns for simple request/response handling.
//!
//! This module provides [`handler_exchange`] and [`handler_exchange_async`],
//! which create exchange handlers that directly process requests without
//! the overhead of component insertion/removal.

use crate::prelude::*;
use beet_core::prelude::*;
use std::sync::Arc;

/// Creates an exchange handler that processes requests directly with a synchronous function.
///
/// Unlike [`spawn_exchange`], this pattern does not insert/remove [`Request`] and [`Response`]
/// components. Instead, the handler function receives the request directly and returns
/// a response, which is sent immediately via the exchange channel.
///
/// ## Execution Flow
///
/// 1. [`ExchangeStart`] is triggered on the spawner entity
/// 2. The handler function is called with [`EntityWorldMut`] and [`Request`]
/// 3. The returned [`Response`] is sent via the exchange channel
///
/// ## Example
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// let mut world = World::new();
/// let mut entity = world.spawn(handler_exchange(|_entity, request| {
///     // Echo the request path back as a response
///     request.mirror()
/// }));
/// ```
pub fn handler_exchange<F>(func: F) -> impl Bundle
where
	F: 'static + Send + Sync + Clone + Fn(EntityWorldMut, Request) -> Response,
{
	OnSpawn::observe(
		move |ev: On<ExchangeStart>, mut commands: Commands| -> Result {
			let func = func.clone();
			let spawner_entity = ev.event_target();
			let (req, cx) = ev.take()?;

			commands.queue(move |world: &mut World| -> Result {
				let res = func(world.entity_mut(spawner_entity), req);
				let mut entity = world.entity_mut(spawner_entity);
				cx.end(&mut entity, res)?;
				Ok(())
			});

			Ok(())
		},
	)
}

/// Creates an exchange handler that processes requests with an async function.
///
/// Unlike [`spawn_exchange`], this pattern does not insert/remove [`Request`] and [`Response`]
/// components. Instead, the handler function receives the request directly and returns
/// a future that resolves to a response, which is sent via the exchange channel.
///
/// The async task is spawned directly on the executor, so this works without
/// needing to update the world. The handler receives the spawner [`Entity`] and [`Request`].
///
/// ## Execution Flow
///
/// 1. [`ExchangeStart`] is triggered on the spawner entity
/// 2. An async task is spawned directly on the executor
/// 3. The handler receives the spawner [`Entity`] and [`Request`]
/// 4. When the future resolves, the [`Response`] is sent via the exchange channel
///
/// ## Example
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// let mut world = World::new();
/// let mut entity = world.spawn(handler_exchange_async(|_entity, request| async move {
///     // Async operations can be performed here
///     request.mirror()
/// }));
/// ```
pub fn handler_exchange_async<F, Fut>(func: F) -> impl Bundle
where
	F: 'static + Send + Sync + Fn(Entity, Request) -> Fut,
	Fut: 'static + Send + Future<Output = Response>,
{
	let func = Arc::new(func);
	OnSpawn::observe(
		move |ev: On<ExchangeStart>, mut commands: AsyncCommands| -> Result {
			let func = func.clone();
			let spawner_entity = ev.event_target();
			let (req, cx) = ev.take()?;

			commands.run(async move |world| {
				let response = func(spawner_entity, req).await;
				let entity = world.entity(spawner_entity);
				entity.with(move |mut entity| {
					cx.end(&mut entity, response).ok();
				});
			});

			Ok(())
		},
	)
}

/// Creates a simple mirror exchange handler that echoes requests back as responses.
///
/// Useful for testing and debugging exchange infrastructure.
pub fn mirror_exchange() -> impl Bundle {
	handler_exchange(|_, req| req.mirror())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn handler_sync_works() {
		World::new()
			.spawn(handler_exchange(|_, req| req.mirror_parts()))
			.exchange(Request::get("/foo"))
			.await
			.path_string()
			.xpect_eq("/foo");
	}

	#[beet_core::test]
	async fn handler_sync_custom_response() {
		World::new()
			.spawn(handler_exchange(|_, _| {
				Response::from_status(StatusCode::ImATeapot)
			}))
			.exchange(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::ImATeapot);
	}

	#[beet_core::test]
	async fn handler_async_works() {
		AsyncPlugin::world()
			.spawn(handler_exchange_async(|_, req| async move {
				req.mirror_parts()
			}))
			.exchange(Request::get("/bar"))
			.await
			.path_string()
			.xpect_eq("/bar");
	}

	#[beet_core::test]
	async fn handler_async_custom_response() {
		AsyncPlugin::world()
			.spawn(handler_exchange_async(|_, _| async move {
				Response::from_status(StatusCode::ImATeapot)
			}))
			.exchange(Request::get("/bar"))
			.await
			.status()
			.xpect_eq(StatusCode::ImATeapot);
	}

	#[beet_core::test]
	async fn mirror_works() {
		World::new()
			.spawn(mirror_exchange())
			.exchange(Request::get("/mirror"))
			.await
			.path_string()
			.xpect_eq("/mirror");
	}
}
