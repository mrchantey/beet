//! # Beet Async
//! 
//! ⚠️ Temporary Crate ⚠️
//! 
//! This crate will eventually be replaced with the upstream bevy_async, currently with two differentiating features:
//! - exclusive world access
//! - wasm support
//!
//! The objective here is to coordinate two participants that want to share World access:
//!
//! - The main Bevy schedule
//! - Futures and async tasks running on other threads
//!
//! This is done through the bridge primitive introduced in this crate.
//!
//! This is a beet-owned vendored copy of the in-progress upstream `bevy_async`
//! crate, rewired onto beet's single `bevy` dependency and carrying beet's
//! exclusive-bridge patch ([`AsyncWorld::exclusive`]). See `agent/plans/bevy_async.md`.
//!
//! Invariants of this crate:
//!
//! - Normal rust safety invariants for &mut World (aliasing)
//! - At most one future has world access at a time
//! - Futures only access the world while the scoped pointer (managed by the bridge driver) is live
//! - `SystemState` is always initialized before use
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

pub use crate::bridge_future::{AsyncSystemState, BridgeError};
pub use crate::bridge_request::async_world_sync_point;
pub use crate::plugin::{AsyncPlugin, AsyncTickBudget, AsyncWorld};
#[cfg(target_arch = "wasm32")]
pub use crate::wasm_tick::set_wasm_tick_hook;

/// The async prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
	#[doc(hidden)]
	pub use crate::{
		AsyncPlugin, AsyncSystemState, AsyncTickBudget, AsyncWorld, BridgeError,
		async_world_sync_point,
	};
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
		other_app
			.add_plugins((TaskPoolPlugin::default(), ScheduleRunnerPlugin::default()));
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
		other_app.update();
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
					match system_state.bridge(MySyncPoint, |_| unreachable!()).await {
						Err(BridgeError::SystemParamValidation(_)) => {
							FAILED_VALIDATION.store(true, Ordering::Relaxed);
						}
						_ => unreachable!("Parameter validation should have failed"),
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
