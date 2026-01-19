use async_channel::Sender;
use async_channel::TrySendError;
use beet_core::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;
// Placeholder, will probs be replaced by bevy Template system
pub trait BundleFunc: 'static + Send + Sync + Clone {
	fn bundle_func(self) -> impl Bundle;
}
impl<F, T> BundleFunc for F
where
	F: 'static + Send + Sync + Clone + FnOnce() -> T,
	T: Bundle,
{
	fn bundle_func(self) -> impl Bundle { self() }
}


pub trait ExchangeTarget {
	fn exchange(
		self,
		request: impl Into<Request>,
	) -> impl MaybeSend + Future<Output = Response>;

	/// Exchange a request and get the response body as a string,
	/// used for testing and debugging
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
				let ev = ExchangeStart::new(entity, request, send);
				world.trigger(ev);
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


/// An EntityEvent triggered on the entity containing the server, ie a [`HttpServer`],
/// for each request received.
///
/// This event is relatively low-level, designed for implementation by exchange patterns rather
/// than individual handlers.
///
/// This event must have exactly one consumer, to call [`Self::take`] and get the
/// Request as well as the sender for providing the response.
///
#[derive(EntityEvent)]
pub struct ExchangeStart {
	#[event_target]
	target: Entity,
	inner: Arc<Mutex<Option<(Request, ExchangeContext)>>>,
}

impl ExchangeStart {
	/// Create a new [`ExchangeStart`],
	/// providing the request and a channel [`Sender`]
	/// for when the exchange is complete
	pub fn new(
		target: Entity,
		request: Request,
		on_response: Sender<Response>,
	) -> Self {
		Self {
			target,
			inner: Arc::new(Mutex::new(Some((
				request,
				ExchangeContext::new(on_response),
			)))),
		}
	}

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
				self.target,
				req.path_string()
			);
		}
	}
}


/// Inner data for an exchange start event,
/// this is required because an EntityEvent is immutable borrow only.
#[derive(Clone, Component)]
pub struct ExchangeContext {
	pub start_time: Instant,
	pub on_response: Sender<Response>,
}

impl ExchangeContext {
	/// Create a new [`ExchangeStart`],
	/// providing the request and a channel [`Sender`]
	/// for when the exchange is complete
	pub fn new(on_response: Sender<Response>) -> Self {
		Self {
			start_time: Instant::now(),
			on_response,
		}
	}


	pub fn end(&self, entity: &mut EntityWorldMut, res: Response) -> Result {
		let id = entity.id();
		entity.world_scope(|world| {
			world.trigger(ExchangeEnd {
				entity: id,
				start_time: self.start_time,
				status: res.status(),
			})
		});
		self.send(res)
	}
	pub fn end_cmd(
		&self,
		entity: &mut EntityCommands,
		res: Response,
	) -> Result {
		let id = entity.id();
		entity.commands_mut().trigger(ExchangeEnd {
			entity: id,
			start_time: self.start_time,
			status: res.status(),
		});
		self.send(res)
	}

	/// Send the response back to the caller,
	/// if the receiver is dropped this is a no-op.
	fn send(&self, response: Response) -> Result {
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

/// Represents the end of an exchange, usually the
/// point at which we exit 'bevy land', hence channels
/// instead of bevy events.
///
/// This may be used as a component but its perfectly valid to use directly
#[derive(Clone, EntityEvent)]
pub struct ExchangeEnd {
	pub entity: Entity,
	pub start_time: Instant,
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
			cx.send(res)
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
					let mut entity = commands.entity(ev.event_target());
					cx.end_cmd(&mut entity, res)
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
