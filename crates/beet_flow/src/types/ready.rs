//! Async readiness signaling for action initialization.
//!
//! This module provides a mechanism for actions to signal when they have
//! completed async initialization. This is useful for dynamically spawned
//! trees that require setup before execution.
use beet_core::prelude::*;

/// Event triggered to request that an entity begin its ready process.
///
/// Actions with async initialization should listen for this event and
/// trigger [`Ready`] when initialization completes.
#[derive(Debug, Component, EntityEvent)]
pub struct GetReady(pub Entity);

/// Event triggered by an entity to signal it has completed initialization.
///
/// This event auto-propagates up the hierarchy, allowing parent actions
/// to await readiness of all descendants.
#[derive(Debug, Component, EntityEvent)]
#[entity_event(auto_propagate)]
pub struct Ready(pub Entity);


/// Marker component for actions that require async initialization.
///
/// Actions marked with this component will be discovered by [`AwaitReady`]
/// and triggered with [`GetReady`]. The action must respond with [`Ready`]
/// when initialization completes.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = World::new();
/// // Create an action that loads data asynchronously
/// world.spawn(ReadyAction::run(async |entity| {
///     // Perform async initialization...
/// }));
/// ```
#[derive(Debug, Component)]
pub struct ReadyAction {
	sealed: PhantomData<()>,
}


impl ReadyAction {
	/// Creates a [`ReadyAction`] that runs an async function on [`GetReady`].
	///
	/// The provided function receives an [`AsyncEntity`] and should perform
	/// any async initialization. [`Ready`] is automatically triggered when
	/// the future completes, regardless of the outcome.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_flow::prelude::*;
	/// # let mut world = World::new();
	/// world.spawn(ReadyAction::run(async |entity| {
	///     // Load assets, connect to services, etc.
	/// }));
	/// ```
	pub fn run<Fut, Out>(
		func: impl 'static + Send + Sync + Clone + FnOnce(AsyncEntity) -> Fut,
	) -> (Self, OnSpawn)
	where
		Fut: 'static + Send + Sync + Future<Output = Out> + Send,
		Out: AsyncTaskOut,
	{
		(
			Self { sealed: default() },
			OnSpawn::observe(
				move |ev: On<GetReady>, mut commands: AsyncCommands| {
					let entity = ev.event_target();
					let observer = func.clone();
					commands.run(async move |world| {
						let out = observer(world.entity(entity)).await;
						world.entity(entity).trigger_then(Ready).await;
						out
					});
				},
			),
		)
	}
	/// Creates a [`ReadyAction`] that runs a `!Send` async function on [`GetReady`].
	///
	/// Similar to [`Self::run`], but the future does not need to be [`Send`].
	/// Useful for WASM or when working with non-thread-safe resources.
	pub fn run_local<Fut, Out>(
		func: impl 'static + Send + Sync + Clone + FnOnce(AsyncEntity) -> Fut,
	) -> (Self, OnSpawn)
	where
		Fut: 'static + Future<Output = Out> + Send,
		Out: AsyncTaskOut,
	{
		(
			Self { sealed: default() },
			OnSpawn::observe(
				move |ev: On<GetReady>, mut commands: AsyncCommands| {
					let entity = ev.event_target();
					let observer = func.clone();
					commands.run_local(async move |world| {
						let out = observer(world.entity(entity)).await;
						world.entity(entity).trigger_then(Ready).await;
						out
					});
				},
			),
		)
	}
}
