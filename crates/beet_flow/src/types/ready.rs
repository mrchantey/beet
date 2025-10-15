use beet_core::exports::async_channel;
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
/// list of entities a parent must await a [`Ready`] signal from
#[derive(Debug, Component)]
pub struct ReadyAction {
	sealed: PhantomData<()>,
}


impl ReadyAction {
	/// Runs the provided method when [`GetReady`] is triggered, and triggers
	/// [`Ready`] upon completion regardless of the outcome.
	pub fn new<Fut, Out>(
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
}


#[extend::ext(name=EntityWorldMutReadyExt)]
pub impl EntityWorldMut<'_> {
	/// Triggers [`GetReady`] for this entity and completes
	/// when the entity triggers [`Ready`].
	fn await_ready(&mut self) -> impl Future<Output = &mut Self> {
		let (send, recv) = async_channel::bounded(1);
		self.observe(move |_: On<Ready>| {
			send.try_send(()).ok();
		})
		.trigger(GetReady)
		.flush();
		async move {
			AsyncRunner::poll_and_update(
				|| {
					self.world_scope(|world| {
						world.update();
					})
				},
				recv,
			)
			.await;
			self
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn await_ready() {
		let store = Store::default();

		let mut world = AsyncPlugin::world();
		world
			.spawn((ReadyAction::new(async move |_| {
				store.set(true);
			}),))
			.await_ready()
			.await;
		store.get().xpect_eq(true);

		AsyncRunner::flush_async_tasks(&mut world).await;
	}
}
