#![doc = include_str!("../README.md")]
//!
//! See `agent/plans/bevy_async.md` for the exclusive-bridge patch
//! ([`AsyncWorld::exclusive`]).
//!
//! Invariants of this crate:
//!
//! - Normal rust safety invariants for &mut World (aliasing)
//! - At most one future has world access at a time
//! - Futures only access the world while the scoped pointer (managed by the bridge driver) is live
//! - `SystemState` is always initialized before use (but a completed request may
//!   carry an *uninitialized* cell if its future re-queued without gaining access,
//!   in which case applying it is a no-op)
//! - Deferred ops are only applied after every future finishes polling and releases world access
//! - The driver can't deadlock
//! - All futures that want world access can eventually complete (assuming fair scheduling by the async runtime)
//! - If the world is dropped, futures don't leak and eventually finish (in an error state)
//!
//!
//! The protocol:
//!
//! Futures (tasks on worker threads)
//! - enqueue requests (create signal guard clones: one kept, one sent)
//!
//! - Driver([`async_world_sync_point`]) (exclusive system, world-owning thread)
//!   1. Drain request queue for this sync point
//!   2. Publish World pointer (via `scoped_static_storage`). Future access scope begins
//!   3. Wake all drained futures
//!
//!  -> Futures race for locks (non-blocking)
//!
//!  -> Success: acquire both locks, do work, complete
//!
//!  -> Failure: signal driver (Drop signal guard), re-enqueue later
//!
//!  -> Direct access: non-queued future polled during scope,
//!  bypasses queue, acquires locks, completes (no signal)
//!   4. Wait for all signal guards to drop + scope mutex released
//!   5. Unpublish pointer, scope ends.
//!   6. Apply any deferred ops from `SystemState` of polled futures
//!   7. Loop (up to [`AsyncTickBudget`]) or return
//!   8. Schedule resumes (normal systems run)
//!
//!
//! Dual locking:
//!
//! The published World pointer lock is managed by the `ScopedStatic` primitive in `scoped_static_storage` (only one future can lock this at a time)
//! `SystemState` locks are managed by the `SystemStateCell` primitive of this crate (futures using different `SystemState` types can work in parallel)
//!
//!
//! Preventing driver deadlocks when futures panic:
//!
//! If a future panics while holding locks, rust's panic unwinding drops destructors in reverse scope order
//! - First, the `SystemState` `MutexGuard` drops (releasing the lock)
//! - Second, the World pointer's scope `MutexGuard` drops (releasing the lock)
//! - Finally, the guard signal constructed by the future during `poll()` drops, and the driver is notified
//!
//! How futures can fail cleanly:
//!
//! If the [`AsyncWorld`] cannot be reached ([`bevy::platform::sync::Weak::upgrade`] fails during `poll()`), the world has been dropped and the future cannot complete.
//!
//! If `SystemState`s are invalid, they can't be used and the future cannot complete
//!
//! Regardless, the future returns Ready(Err) and completes permanently
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

#[cfg(feature = "std")]
extern crate std;

mod bridge_future;
mod bridge_request;
mod plugin;
mod system_state;
mod wake_signal;
#[cfg(target_arch = "wasm32")]
mod wasm_tick;

pub use crate::bridge_future::AsyncSystemState;
pub use crate::bridge_future::BridgeError;
pub use crate::bridge_request::async_world_sync_point;
pub use crate::plugin::AsyncPlugin;
pub use crate::plugin::AsyncTickBudget;
pub use crate::plugin::AsyncWorld;
#[cfg(target_arch = "wasm32")]
pub use crate::wasm_tick::set_wasm_tick_hook;

/// The async prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
	#[doc(hidden)]
	pub use crate::AsyncPlugin;
	#[doc(hidden)]
	pub use crate::AsyncSystemState;
	#[doc(hidden)]
	pub use crate::AsyncTickBudget;
	#[doc(hidden)]
	pub use crate::AsyncWorld;
	#[doc(hidden)]
	pub use crate::BridgeError;
	#[doc(hidden)]
	pub use crate::async_world_sync_point;
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use bevy::app::ScheduleRunnerPlugin;
	use bevy::app::prelude::*;
	use bevy::ecs::prelude::*;
	use bevy::platform::sync::atomic::AtomicBool;
	use bevy::platform::sync::atomic::AtomicI32;
	use bevy::platform::sync::atomic::Ordering;
	use bevy::tasks::AsyncComputeTaskPool;

	/// This tests that if a world is dropped we return an error from attempting to run it and
	/// that everything cleans up nicely
	/// Because of a quirk of how bevy's task pools work we have to always have at least one
	/// active world for anything to progress on them.
	/// That's what `other_app` is for.
	#[test]
	fn dropped_world() {
		struct MySyncPoint;
		static WORLD_WAS_DROPPED: AtomicBool = AtomicBool::new(false);
		let mut other_app = App::new();
		other_app.add_plugins((
			TaskPoolPlugin::default(),
			ScheduleRunnerPlugin::default(),
		));
		let mut app = App::new();
		app.add_plugins((
			AsyncPlugin::default(),
			ScheduleRunnerPlugin::default(),
			TaskPoolPlugin::default(),
		));

		app.add_systems(Startup, move |world: Res<AsyncWorld>| {
			let world = world.clone();
			AsyncComputeTaskPool::get()
				.spawn(async move {
					let system_state = world.system_state::<Commands>();
					match system_state
						.bridge(MySyncPoint, |mut commands: Commands| {
							commands.spawn_empty();
						})
						.await
					{
						Err(BridgeError::WorldDropped) => {
							WORLD_WAS_DROPPED.store(true, Ordering::Relaxed);
						}
						_ => unreachable!("World should have Dropped"),
					}
				})
				.detach();
		});
		app.update();
		drop(app);
		// the detached task only observes the drop once the task pool polls it
		// again; a single tick races its scheduling, so pump until it resolves.
		for _ in 0..1000 {
			other_app.update();
			if WORLD_WAS_DROPPED.load(Ordering::Relaxed) {
				break;
			}
			std::thread::yield_now();
		}
		assert!(WORLD_WAS_DROPPED.load(Ordering::Relaxed));
	}

	#[test]
	fn invalid_parameters() {
		struct MySyncPoint;
		static FAILED_VALIDATION: AtomicBool = AtomicBool::new(false);

		#[derive(Resource)]
		struct MyResource;

		let mut app = App::new();
		app.add_plugins((
			AsyncPlugin::default(),
			ScheduleRunnerPlugin::default(),
			TaskPoolPlugin::default(),
		));

		app.add_systems(Update, async_world_sync_point::<MySyncPoint>);

		app.add_systems(Startup, move |world: Res<AsyncWorld>| {
			let world = world.clone();
			AsyncComputeTaskPool::get()
				.spawn(async move {
					let system_state = world.system_state::<Res<MyResource>>();
					match system_state
						.bridge(MySyncPoint, |_| unreachable!())
						.await
					{
						Err(BridgeError::SystemParamValidation(_)) => {
							FAILED_VALIDATION.store(true, Ordering::Relaxed);
						}
						_ => unreachable!(
							"Parameter validation should have failed"
						),
					}
				})
				.detach();
		});

		app.update();

		assert!(FAILED_VALIDATION.load(Ordering::Relaxed));
	}

	/// Exclusive bridge (beet patch): a future obtains the raw `&mut World`,
	/// mutates structure, and returns a value directly without `SystemState`.
	#[test]
	fn exclusive_access() {
		struct MySyncPoint;
		static SPAWNED: AtomicI32 = AtomicI32::new(-1);

		let mut app = App::new();
		app.add_plugins((
			AsyncPlugin::default(),
			ScheduleRunnerPlugin::default(),
			TaskPoolPlugin::default(),
		));
		app.add_systems(Update, async_world_sync_point::<MySyncPoint>);

		app.add_systems(Startup, move |world: Res<AsyncWorld>| {
			let world = world.clone();
			AsyncComputeTaskPool::get()
				.spawn(async move {
					let count = world
						.exclusive(MySyncPoint, |world: &mut World| {
							world.spawn_empty();
							world.entities().len()
						})
						.await
						.unwrap();
					SPAWNED.store(count as i32, Ordering::Relaxed);
				})
				.detach();
		});

		app.update();

		assert!(SPAWNED.load(Ordering::Relaxed) >= 1);
	}

	/// Regression test for an async-bridge panic (see `agent/plans/async_bridge.md`).
	///
	/// When two futures with *different* `SystemParam` types (hence different
	/// `SystemStateCell`s) are woken in the same sync point, one can grab the
	/// global `world_scope` lock while the other, polled concurrently on a worker
	/// thread, fails to and re-queues. The re-queuing future had already dropped
	/// its wake signal at the top of `poll`, so the driver treated its request as
	/// complete and called `apply` on a `SystemState` that was never initialized,
	/// panicking on `OnceLock::get().unwrap()`.
	///
	/// We widen the contention window by holding the world scope in a busy spin,
	/// and spawn many interleaved tasks of two param types so a concurrent poll
	/// reliably loses the race. Pre-fix the driver panics inside `app.update()`.
	#[test]
	fn concurrent_bridge_does_not_panic_on_uninitialized_state() {
		struct MySyncPoint;
		#[derive(Resource)]
		struct Marker;
		static COMPLETED: AtomicI32 = AtomicI32::new(0);

		const TASKS_PER_KIND: usize = 32;

		let mut app = App::new();
		app.add_plugins((
			AsyncPlugin::default(),
			ScheduleRunnerPlugin::default(),
			TaskPoolPlugin::default(),
		));
		app.insert_resource(Marker);
		app.add_systems(Update, async_world_sync_point::<MySyncPoint>);

		app.add_systems(Startup, move |world: Res<AsyncWorld>| {
			let pool = AsyncComputeTaskPool::get();
			for _ in 0..TASKS_PER_KIND {
				// Kind A holds the world scope in a busy spin, widening the
				// contention window for concurrently-polled kind B futures.
				let world_a = world.clone();
				pool.spawn(async move {
					world_a
						.system_state::<Commands>()
						.bridge(MySyncPoint, |mut commands: Commands| {
							let start = std::time::Instant::now();
							while start.elapsed().as_micros() < 200 {
								std::hint::spin_loop();
							}
							commands.spawn_empty();
						})
						.await
						.unwrap();
					COMPLETED.fetch_add(1, Ordering::Relaxed);
				})
				.detach();

				// Kind B uses a *different* `SystemParam` type, so it owns a
				// separate `SystemStateCell`. This is the future whose cell
				// stayed uninitialized in the original bug.
				let world_b = world.clone();
				pool.spawn(async move {
					world_b
						.system_state::<Res<Marker>>()
						.bridge(MySyncPoint, |_marker: Res<Marker>| {})
						.await
						.unwrap();
					COMPLETED.fetch_add(1, Ordering::Relaxed);
				})
				.detach();
			}
		});

		// Drive until every task completes. Pre-fix, the driver panics here.
		let total = (TASKS_PER_KIND * 2) as i32;
		for _ in 0..1000 {
			app.update();
			if COMPLETED.load(Ordering::Relaxed) == total {
				break;
			}
		}
		assert_eq!(COMPLETED.load(Ordering::Relaxed), total);
	}

	#[test]
	#[cfg(not(feature = "std"))]
	fn no_std_test() {
		use crate::prelude::*;
		use bevy::app::ScheduleRunnerPlugin;
		use bevy::app::prelude::*;
		use bevy::ecs::prelude::*;
		use bevy::platform::sync::atomic::AtomicBool;
		use bevy::platform::sync::atomic::Ordering;
		use bevy::tasks::AsyncComputeTaskPool;

		struct MySyncPoint;
		static ACCESS_RAN: AtomicBool = AtomicBool::new(false);
		let mut app = App::new();
		app.add_plugins((
			AsyncPlugin::default(),
			ScheduleRunnerPlugin::default(),
			TaskPoolPlugin::default(),
		));

		app.add_systems(Update, async_world_sync_point::<MySyncPoint>);

		app.add_systems(Startup, move |world: Res<AsyncWorld>| {
			let world = world.clone();
			AsyncComputeTaskPool::get()
				.spawn_local(async move {
					let system_state = world.system_state::<Commands>();
					system_state
						.bridge(MySyncPoint, |mut commands: Commands| {
							commands.spawn_empty();
							ACCESS_RAN.store(true, Ordering::Relaxed);
						})
						.await
						.unwrap();
				})
				.detach();
		});

		app.update();

		assert!(ACCESS_RAN.load(Ordering::Relaxed));
	}
}
