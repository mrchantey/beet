use crate::bridge_ticker::BridgeTicker;
use crate::plugin::AsyncTickBudget;
use crate::plugin::StrongAsyncWorld;
use crate::system_state::ErasedSystemStateCell;
use crate::wake_signal::WakeSignaler;
use bevy::ecs::prelude::IntoSystemSet;
use bevy::ecs::prelude::SystemSet;
use bevy::ecs::prelude::World;
use bevy::ecs::schedule::InternedSystemSet;
use bevy::platform::prelude::Vec;
use bevy::platform::prelude::vec;
use bevy::platform::sync::Arc;
use bevy::platform::sync::atomic::AtomicBool;
use bevy::platform::sync::atomic::Ordering;
use core::task::Context;

/// Drives the queued bridge work for `SyncPoint`.
///
/// Every queued bridge request is guaranteed to be *woken*. That wake guarantees the corresponding
/// async future gets a chance to poll.
/// It does *not* however guarantee the poll will finish its ECS work, because that
/// poll may still fail to finish it's work for a *variety* of reasons, i.e. it is unable to acquire
/// the typed `SystemState` lock and returns `Poll::Pending`.
///
/// For [`bevy::tasks::TaskPool::spawn_local`] we *are* actually guaranteed that the poll will finish
/// it's ECS work, because it's single threaded, so you can use `spawn_local` if you want
/// determinism.
///
/// This function attempts to tick queued work several times, up to `AsyncTickBudget`.
/// If one internal tick finds no work, we opportunistically tick the local global task pool and
/// try once more before returning early.
///
/// We tick queued work multiple times for two reasons. The first is that serial `.await` calls
/// should try to all be completed within the same `SyncPoint` such as
/// ```rust,ignore
/// let health = task_1.run(|health: Single<&Health, With<Player>>| {
///     health.0
/// }).await;
/// if health == 0 {
///     return;
/// }
/// task_1.run(|commands: Commands| {
///     commands.trigger(PlayerDoesAttack);
/// }).await;
/// ```
/// The second reason is spoken of prior. Poll may fail to finish for a variety of reasons and
/// should be given several chances before giving up.
pub fn async_world_sync_point<SyncPoint: 'static>(world: &mut World) {
	// Derive the stable interned system-set key used to look up requests queued
	// for this exact sync point type.
	let sync_point = async_world_sync_point::<SyncPoint>
		.into_system_set()
		.intern();
	let async_world = world.get_resource::<StrongAsyncWorld>().unwrap().clone();
	// Read the configured maximum number of internal attempts we are willing to
	// perform during this `SyncPoint`.
	let max_ticks = world.get_resource::<AsyncTickBudget>().unwrap().0;
	// the executor serving *this* world, resolved once and reused for every tick
	// below (see `BridgeTicker`).
	let ticker = BridgeTicker::resolve(world);
	for _ in 0..max_ticks {
		// Drive once. If no work was found, we may truly be done.
		// but we should give external task pools one more opportunity to make newly-woken
		// tasks runnable.
		if async_world.0.tick_sync_point(sync_point, world, &ticker)
			== TickResult::NoWork
		{
			ticker.tick();
			// Retry once after ticking. If we are still idle, there is no more
			// immediately available progress to make.
			if async_world.0.tick_sync_point(sync_point, world, &ticker)
				== TickResult::NoWork
			{
				return;
			}
		}
	}
}

#[derive(Default)]
pub(crate) struct AsyncWorldInner {
	pub(crate) bridge_requests:
		keyed_concurrent_queue::KeyedQueues<InternedSystemSet, BridgeRequest>,
	world_scope: scoped_static_storage::ScopedStatic<World>,
	/// Whether `world_scope` currently publishes a world. The pointer sits
	/// behind the scope mutex, which a future holds for the length of its
	/// closure, so a failed `try_with` cannot tell a contended lock from an
	/// unpublished world; this flag, held for a superset of the pointer's
	/// lifetime, answers that in [`park`](Self::park).
	published: AtomicBool,
}

#[cfg(feature = "std")]
std::thread_local! {
	/// The worlds whose bridge closure is running on this thread, by inner
	/// address: a request contending for one of these from a nested tick (the
	/// closure spawns, the spawn ticks, the tick polls a sibling) has the
	/// holder up its own stack, so re-polling could never find it released.
	static HELD: core::cell::RefCell<bevy::platform::prelude::Vec<usize>> =
		const { core::cell::RefCell::new(bevy::platform::prelude::Vec::new()) };
}

impl AsyncWorldInner {
	/// Run `func` against the published world, `None` if none is published or
	/// another closure holds it.
	pub(crate) fn try_with<R>(
		&self,
		func: impl FnOnce(&mut World) -> R,
	) -> Option<R> {
		self.world_scope
			.try_with(|world| {
				#[cfg(feature = "std")]
				let _held = Held::enter(self.key());
				func(world)
			})
			.ok()
	}

	/// Park a future whose [`try_with`](Self::try_with) found no world: a
	/// queued request for the next sync point, or an immediate re-poll while
	/// the world is published on another thread's watch, meaning the scope
	/// lock was merely contended. A queued request only drains once the scope
	/// ends, and a driver ticking nested work (a test suite's sibling futures,
	/// each driving its own world) can hold the scope open for seconds, so
	/// queueing under it starves the future.
	///
	/// Returns the wake signal to hold until the next poll, `None` on a re-poll.
	pub(crate) fn park(
		&self,
		system_set: InternedSystemSet,
		system_state: Arc<dyn ErasedSystemStateCell>,
		cx: &Context<'_>,
	) -> Option<WakeSignaler> {
		if self.contended() {
			cx.waker().wake_by_ref();
			return None;
		}
		let (wake_signal, wake_waiter) = crate::wake_signal::pair();
		self.bridge_requests
			.try_send(&system_set, BridgeRequest {
				waker: cx.waker().clone(),
				wake_waiter,
				system_state,
			})
			.ok()
			.unwrap();
		// the driver may have published and drained between the check above
		// and the push, leaving the request unseen until the scope ends: re-poll
		// to take the direct path instead. The stale entry costs one spurious
		// wake, its signal released by the next poll.
		if self.contended() {
			cx.waker().wake_by_ref();
		}
		Some(wake_signal)
	}

	/// Whether the world is published with its closure running on another
	/// thread, so a re-poll can find it released. A closure up this thread's
	/// own stack cannot finish until the poll returns; without threads it is
	/// the only kind.
	fn contended(&self) -> bool {
		#[cfg(feature = "std")]
		let held_here = HELD.with(|held| held.borrow().contains(&self.key()));
		#[cfg(not(feature = "std"))]
		let held_here = true;
		self.published.load(Ordering::Acquire) && !held_here
	}

	#[cfg(feature = "std")]
	fn key(&self) -> usize { self as *const Self as usize }

	/// Run `func` with `world` published to bridged futures, the pointer's
	/// scope nested inside the flag's so the flag never reads false while the
	/// pointer is set.
	fn publish<R>(&self, world: &mut World, func: impl FnOnce() -> R) -> R {
		struct Unpublish<'a>(&'a AtomicBool);
		impl Drop for Unpublish<'_> {
			fn drop(&mut self) { self.0.store(false, Ordering::Release); }
		}
		self.published.store(true, Ordering::Release);
		let _unpublish = Unpublish(&self.published);
		self.world_scope.scope(world, func)
	}

	/// This ticks a single sync point, requesting the poll of all tasks in that sync point.
	/// None of the tasks are guaranteed to actually return `Poll::Ready`, but all are guaranteed to
	/// at least do a `Poll::Pending`
	///
	/// The flow of logic is the following:
	/// 1. We first drain the queue for our `SyncPoint`.
	/// 2. Expose our `World` through `world_scope`.
	/// 3. Wake all our `BridgeFuture`s.
	/// 4. Apply our `SystemState` back into the `World`. (Things like `Commands`).
	fn tick_sync_point(
		&self,
		sync_point: InternedSystemSet,
		world: &mut World,
		ticker: &BridgeTicker,
	) -> TickResult {
		let mut queued_requests = vec![];
		while let Ok(queued_task_bridge) =
			self.bridge_requests.get_or_create(&sync_point).pop()
		{
			queued_requests.push(queued_task_bridge);
		}
		// If no requests were waiting then report idle so the caller can decide whether to stop
		// or attempt one more task-pool tick.
		if queued_requests.is_empty() {
			return TickResult::NoWork;
		}
		// Make this `World` temporarily visible to our waking futures. Wake them all and wait
		// until they all have at least *attempted* to poll.
		// This is contractually obligated by the contract of `.wake()`. We are guaranteed one wake
		// per call to our `.wake()`.
		let completed_tasks = self
			.publish(world, || wake_requests_and_wait(queued_requests, ticker));
		for task in completed_tasks {
			task.apply(world);
		}
		TickResult::DidWork
	}
}

/// Marks a world's closure as running on the current thread for the length of
/// the closure (see [`AsyncWorldInner::contended`]).
#[cfg(feature = "std")]
struct Held(usize);

#[cfg(feature = "std")]
impl Held {
	fn enter(key: usize) -> Self {
		HELD.with(|held| held.borrow_mut().push(key));
		Self(key)
	}
}

#[cfg(feature = "std")]
impl Drop for Held {
	fn drop(&mut self) {
		HELD.with(|held| {
			let mut held = held.borrow_mut();
			if let Some(index) = held.iter().rposition(|key| *key == self.0) {
				held.swap_remove(index);
			}
		});
	}
}

/// We need to notify all our Wakers that have queued that we've dropped so they can error
impl Drop for AsyncWorldInner {
	fn drop(&mut self) {
		for bridge_requests in
			self.bridge_requests.inner().read().unwrap().values()
		{
			while let Ok(request) = bridge_requests.pop() {
				request.waker.wake();
			}
		}
	}
}

/// Whether a tick attempt made any progress.
#[derive(PartialEq)]
enum TickResult {
	/// We found and processed at least one queued bridge request.
	DidWork,
	/// There was no queued work available for the `SyncPoint`.
	NoWork,
}

/// A queued access request bridging an async task into ECS.
pub(crate) struct BridgeRequest {
	/// Waker for the async future (`crate::bridge_future::BridgeFuture`) that wants ECS access.
	/// When the `SyncPoint` is driven, this waker is fired so the future can
	/// poll while `world_scope` exposes the current `World`.
	pub(crate) waker: core::task::Waker,
	/// Our custom primitive that lets us wait until all the futures have tried to run before
	/// continuing.
	pub(crate) wake_waiter: crate::wake_signal::WakeWaiter,
	pub(crate) system_state: Arc<dyn ErasedSystemStateCell>,
}

/// A queued bridge request whose waker has already been fired.
struct WokenBridgeRequest {
	wake_signal: crate::wake_signal::WakeWaiter,
	system_state: Arc<dyn ErasedSystemStateCell>,
}

/// A request that has finished its attempted poll and may need to apply deferred world state.
struct CompletedBridgeRequest {
	system_state: Arc<dyn ErasedSystemStateCell>,
}

impl CompletedBridgeRequest {
	#[inline]
	fn apply(self, world: &mut World) { self.system_state.apply(world); }
}

#[inline]
fn wake_requests_and_wait(
	queued_requests: Vec<BridgeRequest>,
	ticker: &BridgeTicker,
) -> Vec<CompletedBridgeRequest> {
	let bridged_futures = queued_requests
		.into_iter()
		.map(
			|BridgeRequest {
			     system_state,
			     waker,
			     wake_waiter: wake_signal,
			 }| {
				// Trigger the `BridgeFuture` so it can poll while `world_scope`
				// is active.
				waker.wake();
				WokenBridgeRequest {
					system_state,
					wake_signal,
				}
			},
		)
		// we re-collect to ensure we fully exhaust the prior iterator
		// we want to have all the wakers call .wake() before the first barrier calls .wait()
		.collect::<Vec<_>>();

	ticker.tick();

	bridged_futures
		.into_iter()
		.map(
			|WokenBridgeRequest {
			     system_state,
			     wake_signal,
			 }| {
				wake_signal.wait();
				CompletedBridgeRequest { system_state }
			},
		)
		.collect()
}
