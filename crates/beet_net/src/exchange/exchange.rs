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



#[extend::ext(name=EntityWorldMutExchangeExt)]
pub impl EntityWorldMut<'_> {
	fn exchange(
		&mut self,
		request: Request,
	) -> impl Send + Future<Output = Response> {
		exchange_inner(request, |ctx| {
			self.trigger(ctx);
			// flush any commands created by observers, ie SpawnExchange
			self.world_scope(World::flush);
		})
	}
}

#[extend::ext(name=AsyncEntityExchangeExt)]
pub impl AsyncEntity {
	fn exchange(
		&self,
		request: Request,
	) -> impl Send + Future<Output = Response> {
		exchange_inner(request, |ctx| {
			self.trigger(ctx);
			// no need to flush in async context
		})
	}
}

async fn exchange_inner(
	request: Request,
	trigger: impl FnOnce(ExchangeContext),
) -> Response {
	let (send, recv) = async_channel::bounded(1);
	trigger(ExchangeContext::new(request, send));
	match recv.recv().await {
		Ok(response) => response,
		Err(e) => {
			// if the receiver errors, we can assume the exchange was not handled
			error!("Exchange sender was dropped: {}", e);
			Response::internal_error()
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
	inner: Arc<Mutex<Option<ExchangeContext>>>,
}

impl Drop for ExchangeStart {
	fn drop(&mut self) {
		let mut inner = self.inner.lock().unwrap();
		if let Some(exchange_inner) = inner.take() {
			// TODO custom trigger so we can gracefully error
			panic!(
				"ExchangeStart for entity {:?} was dropped without being handled. \nRequest: {}",
				self.target,
				exchange_inner.request.path_string()
			);
		}
	}
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
			inner: Arc::new(Mutex::new(Some(ExchangeContext {
				request,
				end: ExchangeEnd {
					start_time: Instant::now(),
					send: on_response,
				},
			}))),
		}
	}

	pub fn take(&self) -> Result<ExchangeContext> {
		let mut inner = self.inner.lock().unwrap();
		inner
			.take()
			.ok_or_else(|| bevyhow!("ExchangeInner has already been taken, are there multiple exchange listeners on this entity?"))
	}
}
/// Inner data for an exchange start event,
/// this is required because an EntityEvent is immutable borrow only.
pub struct ExchangeContext {
	pub request: Request,
	pub end: ExchangeEnd,
}

impl ExchangeContext {
	/// Create a new [`ExchangeStart`],
	/// providing the request and a channel [`Sender`]
	/// for when the exchange is complete
	pub fn new(request: Request, on_response: Sender<Response>) -> Self {
		Self {
			request,
			end: ExchangeEnd {
				start_time: Instant::now(),
				send: on_response,
			},
		}
	}
}

/// Allow for entity.trigger
#[cfg(feature = "nightly")]
impl FnOnce<(Entity,)> for ExchangeContext {
	type Output = ExchangeStart;
	extern "rust-call" fn call_once(self, args: (Entity,)) -> Self::Output {
		ExchangeStart {
			target: args.0,
			inner: Arc::new(Mutex::new(Some(self))),
		}
	}
}


// struct MyComplexEvent {
// 	target: Entity,
// 	inner: MyComplexInner,
// }
// struct MyComplexInner {}

// #[cfg(feature = "nightly")]
// impl FnOnce<(Entity,)> for MyComplexInner {
// 	type Output = MyComplexEvent;
// 	extern "rust-call" fn call_once(self, args: (Entity,)) -> Self::Output {
// 		MyComplexEvent {
// 			target: args.0,
// 			inner: self,
// 		}
// 	}
// }

/// Represents the end of an exchange, usually the
/// point at which we exit 'bevy land', hence channels
/// instead of bevy events.
///
/// This may be used as a component but its perfectly valid to use directly
#[derive(Component)]
pub struct ExchangeEnd {
	start_time: Instant,
	send: Sender<Response>,
}

impl ExchangeEnd {
	// fn new(send: Sender<Response>) -> Self {
	// 	Self {
	// 		start_time: Instant::now(),
	// 		send,
	// 	}
	// }
	pub fn start_time(&self) -> Instant { self.start_time }

	/// Send the response back to the caller,
	/// if the receiver is dropped this is a no-op.
	pub fn send(&self, response: Response) -> Result {
		match self.send.try_send(response) {
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
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	#[should_panic = "dropped without being handled"]
	fn unhandled_panics_raw() {
		let mut world = World::new();
		let (send, _) = async_channel::unbounded();
		world
			.spawn_empty()
			.trigger(ExchangeContext::new(Request::get("foo"), send));
	}

	#[beet_core::test]
	#[should_panic = "dropped without being handled"]
	async fn unhandled_panics() {
		let mut world = World::new();
		world.spawn_empty().exchange(Request::get("foo")).await;
	}
	#[beet_core::test]
	#[should_panic = "ExchangeListener has already been taken"]
	async fn double_take() {
		let handler = |ev: On<ExchangeStart>| {
			let req = ev.take().unwrap();
			let res = req.request.mirror();
			req.end.send(res)
		};
		World::new()
			.spawn((OnSpawn::observe(handler), OnSpawn::observe(handler)))
			.exchange(Request::get("foo"))
			.await;
	}

	#[beet_core::test]
	async fn works() {
		World::new()
			.spawn(OnSpawn::observe(|ev: On<ExchangeStart>| {
				let req = ev.take().unwrap();
				let res = req.request.mirror();
				req.end.send(res)
			}))
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
