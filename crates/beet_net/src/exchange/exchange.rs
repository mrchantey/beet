//! Core exchange types for request/response handling.
//!
//! This module provides the fundamental types for entity-based exchanges:
//! [`ExchangeStart`], [`ExchangeContext`], and [`ExchangeEnd`].
use async_channel::Sender;
use async_channel::TrySendError;
use beet_core::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;
/// Trait for types that can produce a [`Bundle`] via a function call.
///
/// This is a placeholder that may be replaced by Bevy's Template system.
pub trait BundleFunc: 'static + Send + Sync + Clone {
	/// Produces the bundle.
	fn bundle_func(self) -> impl Bundle;
}
impl<F, T> BundleFunc for F
where
	F: 'static + Send + Sync + Clone + FnOnce() -> T,
	T: Bundle,
{
	fn bundle_func(self) -> impl Bundle { self() }
}


/// Types that can handle request/response exchanges.
///
/// This trait enables entities and async entity references to participate
/// in HTTP-like exchanges within the Bevy ECS.
pub trait ExchangeTarget {
	/// Sends a request and awaits the response.
	fn exchange(
		self,
		request: impl Into<Request>,
	) -> impl MaybeSend + Future<Output = Response>;

	/// Exchanges a request and returns the response body as a string.
	///
	/// Convenience method for testing and debugging.
	fn exchange_str(
		self,
		request: impl Into<Request>,
	) -> impl MaybeSend + Future<Output = String>
	where
		Self: Sized,
	{
		let fut = self.exchange(request);
		async move { fut.await.unwrap_str().await }
	}
}

impl ExchangeTarget for &mut EntityWorldMut<'_> {
	fn exchange(
		self,
		request: impl Into<Request>,
	) -> impl Future<Output = Response> {
		let entity = self.id();
		let world = unsafe { self.world_mut() };
		let (send, recv) = async_channel::bounded(1);
		let ev = ExchangeStart::new(entity, request.into(), send);
		world.trigger(ev);
		// flush any commands created by observers, ie SpawnExchange
		world.flush();

		// check if response was sent synchronously
		async move {
			match recv.try_recv() {
				Ok(response) => response,
				Err(_) => {
					// poll async tasks until we get a response
					AsyncRunner::poll_and_update(|| world.update_local(), recv)
						.await
				}
			}
		}
	}
}

impl ExchangeTarget for &AsyncEntity {
	fn exchange(
		self,
		request: impl Into<Request>,
	) -> impl MaybeSend + Future<Output = Response> {
		let entity = self.id();
		let world = self.world().clone();
		let request = request.into();
		async move {
			let (send, recv) = async_channel::bounded(1);
			world.with(move |world: &mut World| {
				world.trigger(ExchangeStart::new(entity, request, send));
				world.flush();
			});
			match recv.recv().await {
				Ok(response) => response,
				Err(e) => {
					error!("Exchange sender was dropped: {}", e);
					Response::internal_error()
				}
			}
		}
	}
}


/// Event triggered on a server entity when a request arrives.
///
/// This is a low-level event designed for implementation by exchange patterns
/// (like [`SpawnExchange`] or [`HandlerExchange`]) rather than individual handlers.
///
/// # Important
///
/// This event must have exactly one consumer that calls [`Self::take`] to obtain
/// the request and response sender. Multiple consumers will cause a panic.
#[derive(EntityEvent)]
pub struct ExchangeStart {
	#[event_target]
	server: Entity,
	inner: Arc<Mutex<Option<(Request, ExchangeContext)>>>,
}

impl ExchangeStart {
	/// Creates a new [`ExchangeStart`] with the request and response channel.
	pub fn new(
		server: Entity,
		request: Request,
		on_response: Sender<Response>,
	) -> Self {
		Self {
			server,
			inner: Arc::new(Mutex::new(Some((
				request,
				ExchangeContext::new(on_response),
			)))),
		}
	}

	/// Takes the request and context from this event.
	///
	/// # Errors
	///
	/// Returns an error if called more than once, as the inner data
	/// can only be taken by a single handler.
	pub fn take(&self) -> Result<(Request, ExchangeContext)> {
		let mut inner = self.inner.lock().unwrap();
		inner
			.take()
			.ok_or_else(|| bevyhow!("ExchangeInner has already been taken, are there multiple exchange listeners on this entity?"))
	}
}

impl Drop for ExchangeStart {
	fn drop(&mut self) {
		let mut inner = self.inner.lock().unwrap();
		if let Some((req, _)) = inner.take() {
			// TODO custom trigger so we can gracefully error
			panic!(
				"ExchangeStart for entity {:?} was dropped without being handled. \nRequest: {}",
				self.server,
				req.path_string()
			);
		}
	}
}


/// Context for an in-progress exchange.
///
/// Contains timing information and the response channel. This type is separate
/// from [`ExchangeStart`] because entity events provide immutable access only.
#[derive(Clone, Component)]
pub struct ExchangeContext {
	/// When the exchange started, for timing metrics.
	pub start_time: Instant,
	/// Channel to send the response back to the caller.
	pub on_response: Sender<Response>,
}

impl ExchangeContext {
	/// Creates a new context with the given response channel.
	pub fn new(on_response: Sender<Response>) -> Self {
		Self {
			start_time: Instant::now(),
			on_response,
		}
	}


	/// Ends the exchange, triggers [`ExchangeEnd`], despawns the entity, and sends the response.
	pub fn end(&self, mut entity: EntityWorldMut, res: Response) -> Result {
		let id = entity.id();
		entity.world_scope(|world| {
			world.trigger(ExchangeEnd {
				entity: id,
				start_time: self.start_time,
				status: res.status(),
			});
			world.flush();
		});
		entity.despawn();
		self.end_no_entity(res)
	}
	/// Ends the exchange from an [`EntityCommands`] context and despawns the entity.
	pub fn end_cmd(&self, mut entity: EntityCommands, res: Response) -> Result {
		let id = entity.id();
		entity.commands_mut().trigger(ExchangeEnd {
			entity: id,
			start_time: self.start_time,
			status: res.status(),
		});
		entity.despawn();
		self.end_no_entity(res)
	}

	/// Sends the response without triggering [`ExchangeEnd`] or despawning.
	///
	/// Use this when there is no exchange entity to clean up, or when you
	/// need to handle cleanup separately. If the receiver was dropped,
	/// this is a no-op (returns `Ok`).
	pub fn end_no_entity(&self, response: Response) -> Result {
		match self.on_response.try_send(response) {
			Ok(_) => Ok(()),
			Err(TrySendError::Full(_)) => {
				bevybail!("Response already sent")
			}
			Err(TrySendError::Closed(_)) => {
				// dont panic, if receiver was dropped this implies
				// the caller is no longer interested
				Ok(())
			}
		}
	}
}

/// Event triggered when an exchange completes.
///
/// Contains timing and status information for metrics and logging.
/// This event marks the point at which processing leaves "bevy space"
/// and the response is sent back through channels.
#[derive(Clone, EntityEvent)]
pub struct ExchangeEnd {
	/// The entity that handled this exchange.
	pub entity: Entity,
	/// When the exchange started.
	pub start_time: Instant,
	/// The HTTP status code of the response.
	pub status: StatusCode,
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	#[should_panic = "dropped without being handled"]
	async fn unhandled_panics() {
		let mut world = World::new();
		world.spawn_empty().exchange(Request::get("foo")).await;
	}
	#[beet_core::test]
	#[should_panic = "ExchangeInner has already been taken"]
	async fn double_take() {
		let handler = |ev: On<ExchangeStart>| {
			let (req, cx) = ev.take().unwrap();
			let res = req.mirror();
			cx.end_no_entity(res)
		};
		World::new()
			.spawn((OnSpawn::observe(handler), OnSpawn::observe(handler)))
			.exchange(Request::get("foo"))
			.await;
	}

	#[beet_core::test]
	async fn works() {
		World::new()
			.spawn(OnSpawn::observe(
				|ev: On<ExchangeStart>, mut commands: Commands| {
					let (req, cx) = ev.take().unwrap();
					let res = req.mirror();
					let entity = commands.entity(ev.event_target());
					cx.end_cmd(entity, res)
				},
			))
			.exchange(Request::get("foo"))
			.await
			.status()
			.is_ok()
			.xpect_true();
		// let start = Instant::now();
		// roundtrip should be ~10us when run in isolation
		// start.elapsed().xpect_less_than(Duration::from_micros(100));
		// println!("Handled request in {:?}", start.elapsed());
	}
}
