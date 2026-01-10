use beet_core::prelude::*;

/// Event triggered to indicate that an entity is preparing to become ready.
#[derive(Debug, Component, EntityEvent)]
pub struct GetReady(pub Entity);

// impl RunEvent for GetReady {
// 	type End = Ready;
// }


/// Triggered by an entity in response to [`GetReady`]
#[derive(Debug, Component, EntityEvent)]
#[entity_event(auto_propagate)]
pub struct Ready(pub Entity);


// impl EndEvent for Ready {
// 	type Run = GetReady;
// }

/// Marker to indicate this entity should be included in the
/// list of entities a parent must await a [`Ready`] signal from.
/// See
#[derive(Debug, Component)]
pub struct ReadyAction {
	sealed: PhantomData<()>,
}


impl ReadyAction {
	/// Runs the provided method when [`GetReady`] is triggered, and triggers
	/// [`Ready`] upon completion regardless of the outcome.
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
						world.entity(entity).trigger(Ready).await;
						out
					});
				},
			),
		)
	}
	/// Runs the provided method when [`GetReady`] is triggered, and triggers
	/// [`Ready`] upon completion regardless of the outcome.
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
						world.entity(entity).trigger(Ready).await;
						out
					});
				},
			),
		)
	}
}
