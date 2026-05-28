use crate::bridge_request::BridgeRequest;
use crate::plugin::AsyncWorld;
use crate::system_state::{
	ErasedSystemStateCell, NoopSystemStateCell, SystemStateCell,
};
use crate::wake_signal::WakeSignaler;
use crate::{bridge_request, wake_signal};
use bevy::ecs::schedule::{InternedSystemSet, IntoSystemSet, SystemSet};
use bevy::ecs::system::SystemParam;
use bevy::ecs::world::World;
use bevy::platform::sync::Arc;
use core::marker::PhantomData;

/// Handle that lets an async task request temporary access to an ECS
/// `SystemParam` or a tuple of them.
///
/// `P` is the typed system parameter the caller eventually wants, such as:
/// - [`bevy::ecs::prelude::Commands`]
/// - [`bevy::ecs::prelude::Res`]
/// - [`bevy::ecs::prelude::Query`]
/// - tuples of params
///
/// It is cheap to clone and intended to be passed into async tasks.
/// You can pass it into *multiple* tasks on separate threads and have them work concurrently
/// off of the same state, sharing `Locals`.
pub struct AsyncSystemState<P: SystemParam + 'static> {
	// `fn() -> P` so the handle's `Send`/`Sync` does not depend on `P`
	// (the real state lives in `system_state`, which is `Send + Sync`).
	pub(crate) _p: PhantomData<fn() -> P>,

	/// A `Weak` is used so tasks do not stay alive if the world is dropped.
	/// If the world goes away, upgrading this weak pointer fails and access
	/// returns [`BridgeError::WorldDropped`].
	pub(crate) world: AsyncWorld,

	/// Type-erased storage for the underlying `SystemState<P>`.
	///
	/// Each `AsyncSystemState<P>` keeps reusing the same typed system state across
	/// accesses so repeated operations do not rebuild it from scratch.
	///
	/// This is also important not only to persist params like `Local` but *also* so `Changed` and
	/// `Added` and other filters can work.
	pub(crate) system_state: Arc<dyn ErasedSystemStateCell>,
}

impl<P: SystemParam + 'static> Clone for AsyncSystemState<P> {
	fn clone(&self) -> Self {
		Self {
			_p: PhantomData,
			world: self.world.clone(),
			system_state: self.system_state.clone(),
		}
	}
}

impl<P: SystemParam + 'static> AsyncSystemState<P> {
	/// Create a new `AsyncSystemState` from an `AsyncWorld` matching the Api surface of
	/// `SystemState` with `World`.
	pub fn new(world: AsyncWorld) -> Self {
		Self {
			_p: PhantomData,
			world,
			#[cfg(feature = "std")]
			system_state: Arc::new(SystemStateCell::<P>::default()),
			#[cfg(not(feature = "std"))]
			system_state: Arc::from(
				bevy::platform::prelude::Box::new(
					SystemStateCell::<P>::default(),
				) as bevy::platform::prelude::Box<dyn ErasedSystemStateCell>,
			),
		}
	}

	/// This function allows us to create a bridge between the async task we are in and the ecs
	/// world we want access to, effectively running a system from an async task. The systems run
	/// here are able to take in `&` and `&mut` variables from the surrounding context unlike
	/// standard Bevy systems.
	///
	/// We bridge *at* the `_sync_point` `SyncPoint` with our `bridge_fn`.
	pub async fn bridge<BridgeFn, Out, SyncPoint: 'static>(
		&self,
		_sync_point: SyncPoint,
		bridge_fn: BridgeFn,
	) -> Result<Out, BridgeError>
	where
		for<'w, 's> BridgeFn: FnOnce(P::Item<'w, 's>) -> Out,
	{
		BridgeFuture {
			_p: PhantomData,
			system_set: bridge_request::async_world_sync_point::<SyncPoint>
				.into_system_set()
				.intern(),
			bridge_fn: Some(bridge_fn),
			wake_signal: None,
			system_state: self.system_state.clone(),
			world: self.world.clone(),
		}
		.await
	}
}

impl AsyncWorld {
	/// Bridge directly to the raw `&mut World` at `_sync_point`, bypassing
	/// `SystemState` entirely (beet patch, a divergence from upstream `bevy_async`).
	///
	/// Because [`scoped_static_storage::ScopedStatic::try_with`] hands the closure
	/// a real `&mut World` (mutex-serialized to one future at a time), this lets us
	/// run arbitrary exclusive world mutations and return their output directly,
	/// without the `SystemState`/deferred-apply layer. The queued request carries a
	/// [`NoopSystemStateCell`] so the existing driver wakes/serves it unchanged.
	pub fn exclusive<Func, Out, SyncPoint: 'static>(
		&self,
		_sync_point: SyncPoint,
		func: Func,
	) -> impl Future<Output = Result<Out, BridgeError>>
	where
		for<'w> Func: FnOnce(&'w mut World) -> Out,
	{
		ExclusiveBridgeFuture {
			system_set: bridge_request::async_world_sync_point::<SyncPoint>
				.into_system_set()
				.intern(),
			bridge_fn: Some(func),
			wake_signal: None,
			#[cfg(feature = "std")]
			system_state: Arc::new(NoopSystemStateCell),
			#[cfg(not(feature = "std"))]
			system_state: Arc::from(
				bevy::platform::prelude::Box::new(NoopSystemStateCell)
					as bevy::platform::prelude::Box<dyn ErasedSystemStateCell>,
			),
			world: self.clone(),
		}
	}
}

/// If the bridge cannot run, either because the system params were invalid, or because the world it
/// was referencing no longer exists, we return this error.
#[derive(thiserror::Error, Debug)]
pub enum BridgeError {
	/// The requested `SystemParam` was invalid in the current world context.
	/// for example trying to access a param that fails Bevy's usual validation like a missing
	/// Resource or using `Single` on something that has 0 or multiple instances.
	#[error(transparent)]
	SystemParamValidation(bevy::ecs::system::SystemParamValidationError),
	/// The world has been dropped, so we should just return.
	#[error("World no longer exists")]
	WorldDropped,
}

/// Future representing a single in-flight bridging request between our async task and our `World`.
struct BridgeFuture<P: SystemParam + 'static, Func, Out> {
	// `fn() -> (P, Out)` so the future's `Send` depends only on its real fields
	// (`bridge_fn: Option<Func>`, the `Send + Sync` system state, etc.), not on
	// the phantom `P`/`Out` markers.
	_p: PhantomData<fn() -> (P, Out)>,
	/// Interned system-set key identifying which sync-point queue this future
	/// should be sent to.
	system_set: InternedSystemSet,
	/// This is the pseudo-system that we try to run when we have access to `World`.
	/// This is an option just so we can take it out when we run it so we can use `FnOnce`
	/// instead of `FnMut`, so it's more flexible than true systems.
	bridge_fn: Option<Func>,
	/// Wake signal for the currently queued wake cycle, if any.
	///
	/// The future drops this at the end of `poll` which acts as acknowledgement that the wake
	/// has been handled.
	wake_signal: Option<WakeSignaler>,
	system_state: Arc<dyn ErasedSystemStateCell>,
	/// Weak bridge pointer so the loss of the world becomes a clean runtime error.
	world: AsyncWorld,
}

impl<P: SystemParam + 'static, Func, Out> Unpin for BridgeFuture<P, Func, Out> {}

impl<P, Func, Out> Future for BridgeFuture<P, Func, Out>
where
	P: SystemParam + 'static,
	for<'w, 's> Func: FnOnce(P::Item<'w, 's>) -> Out,
{
	type Output = Result<Out, BridgeError>;

	fn poll(
		mut self: core::pin::Pin<&mut Self>,
		cx: &mut core::task::Context<'_>,
	) -> core::task::Poll<Self::Output> {
		use core::task::Poll;

		// If we were previously woken by the sync-point driver, we will have a
		// `WakeSignaler` stored here.
		//
		// Dropping that signal at the end of this poll acts as the
		// acknowledgement that yes, this wake was observed and this task has
		// attempted its run, you may release the waiting on the other side.
		let _drop_at_end_of_scope = self.wake_signal.take();

		// Try to gain a strong reference to the bridge. If this fails, the world is gone,
		// so further access is impossible.
		let Some(strong_world) = self.world.0.upgrade() else {
			return Poll::Ready(Err(BridgeError::WorldDropped));
		};
		match strong_world
			.world_scope
			.try_with(|world| {
				let Self {
					ref system_state,
					ref mut bridge_fn,
					..
				} = *self;
				// Attempt to acquire the typed `SystemState<P>`.
				//
				// We deliberately use `try_lock` rather than blocking. If
				// another bridge request is currently using the same system
				// state, we simply yield and let the sync-point driver try again
				// on a later internal tick.
				let Some(mut system_state) = system_state.try_lock::<P>(world)
				else {
					return Poll::Pending;
				};

				if !system_state.meta().is_send() {
					return Poll::Ready(Err(BridgeError::SystemParamValidation(
						bevy::ecs::system::SystemParamValidationError::invalid::<
							bevy::ecs::prelude::NonSend<()>,
						>(
							"Cannot have your system be non-send / exclusive",
						),
					)));
				}

				let param = match system_state.get_mut(world) {
					Ok(param) => param,
					Err(system_param_validation_error) => {
						return Poll::Ready(Err(
							BridgeError::SystemParamValidation(
								system_param_validation_error,
							),
						));
					}
				};
				// We finally have `P::Item<'w, 's>`, yay!, so consume the stored `FnOnce`, run it,
				// and complete the future.
				Poll::Ready(Ok(bridge_fn.take().unwrap()(param)))
			})
			.ok()
		{
			Some(out) => out,
			None => {
				// No world is currently exposed. That means we are being polled
				// outside the `async_world_sync_point`, so we cannot access ECS yet.
				//
				// Instead, enqueue ourselves to be revisited when the matching
				// sync-point system runs.
				let (wake_signal, wake_waiter) = wake_signal::pair();
				// Store the wake_signal locally so dropping it at the end of the next
				// poll acknowledges the wake.
				self.wake_signal.replace(wake_signal);
				// Queue the request under this future's target sync point.
				//
				// The queued payload carries the following!
				// 1. The task's waker, so the sync-point driver can wake it.
				// 2. The wake handshake signal, so the driver can wait until the wake has actually
				// been processed.
				// 3. The erased `SystemState` storage itself.
				strong_world
					.bridge_requests
					.try_send(
						&self.system_set,
						BridgeRequest {
							waker: cx.waker().clone(),
							wake_waiter,
							system_state: self.system_state.clone(),
						},
					)
					.ok()
					.unwrap();
				Poll::Pending
			}
		}
	}
}

/// Exclusive counterpart to [`BridgeFuture`] (beet patch).
///
/// Runs `FnOnce(&mut World) -> Out` directly inside the published `world_scope`,
/// bypassing `SystemState`. See [`AsyncWorld::exclusive`]. `Out` is inferred from
/// `Func` in the [`Future`] impl, so it is not a struct parameter — this keeps the
/// future `Send` whenever `Func` is, independent of `Out`.
struct ExclusiveBridgeFuture<Func> {
	system_set: InternedSystemSet,
	bridge_fn: Option<Func>,
	wake_signal: Option<WakeSignaler>,
	system_state: Arc<dyn ErasedSystemStateCell>,
	world: AsyncWorld,
}

impl<Func> Unpin for ExclusiveBridgeFuture<Func> {}

impl<Func, Out> Future for ExclusiveBridgeFuture<Func>
where
	for<'w> Func: FnOnce(&'w mut World) -> Out,
{
	type Output = Result<Out, BridgeError>;

	fn poll(
		mut self: core::pin::Pin<&mut Self>,
		cx: &mut core::task::Context<'_>,
	) -> core::task::Poll<Self::Output> {
		use core::task::Poll;

		// Acknowledge any prior wake by dropping its signal at the end of poll.
		let _drop_at_end_of_scope = self.wake_signal.take();

		let Some(strong_world) = self.world.0.upgrade() else {
			return Poll::Ready(Err(BridgeError::WorldDropped));
		};
		// `try_with` hands us a real `&mut World` while the driver has published it.
		// On success we run the closure and complete; otherwise the closure is
		// returned and `.ok()` drops it, releasing the borrow on `self`.
		match strong_world
			.world_scope
			.try_with(|world| {
				let Self {
					ref mut bridge_fn, ..
				} = *self;
				bridge_fn.take().unwrap()(world)
			})
			.ok()
		{
			Some(out) => Poll::Ready(Ok(out)),
			None => {
				// World is not published yet: enqueue and wait for the sync point.
				let (wake_signal, wake_waiter) = wake_signal::pair();
				self.wake_signal.replace(wake_signal);
				strong_world
					.bridge_requests
					.try_send(
						&self.system_set,
						BridgeRequest {
							waker: cx.waker().clone(),
							wake_waiter,
							system_state: self.system_state.clone(),
						},
					)
					.ok()
					.unwrap();
				Poll::Pending
			}
		}
	}
}
