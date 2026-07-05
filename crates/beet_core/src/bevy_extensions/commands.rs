//! Extension methods for Bevy's [`Commands`].

use crate::prelude::*;
use bevy::ecs::system::command;
use core::panic::Location;

/// Extension trait adding utility methods to [`Commands`].
#[extend::ext(name=CommandsExt)]
pub impl Commands<'_, '_> {
	/// Logs the names of all components on the given entity.
	fn log_component_names(&mut self, entity: Entity) {
		self.queue(move |world: &mut World| {
			world.log_component_names(entity);
		});
	}

	/// Loads world serde data from [`MediaBytes`].
	#[cfg(feature = "template_serde")]
	fn load_template(&mut self, bytes: impl Into<MediaBytes>) {
		let bytes = bytes.into();
		self.queue(move |world: &mut World| -> Result {
			TemplateLoader::new(world).load(&bytes)?;
			Ok(())
		});
	}

	/// Queues an asynchronous task to be run in the world context.
	#[cfg(feature = "bevy_async")]
	fn queue_async<Func, Fut, Out>(&mut self, func: Func)
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.queue(move |world: &mut World| {
			world.run_async(move |world| func(world));
		});
	}

	/// Queues a local asynchronous task to be run in the world context.
	#[cfg(feature = "bevy_async")]
	fn queue_async_local<Func, Fut, Out>(&mut self, func: Func)
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.queue(move |world: &mut World| {
			world.run_async_local(move |world| func(world));
		});
	}

	/// Runs a system once with the given input.
	fn run_system_once_with<I, M, S>(
		&mut self,
		system: S,
		input: I::Inner<'static>,
	) where
		I: SystemInput<Inner<'static>: Send> + Send + 'static,
		M: 'static,
		S: IntoSystem<I, (), M> + Send + 'static,
	{
		self.queue(move |world: &mut World| {
			world.run_system_once_with(system, input).ok();
		});
	}
}

/// Extension trait adding utility methods to [`EntityCommands`].
#[extend::ext(name=EntityCommandsExt)]
pub impl EntityCommands<'_> {
	/// Triggers an entity event on this entity,
	/// discarding the error if any.
	fn try_trigger<'t, E: EntityEvent<Trigger<'t>: Default>>(
		&mut self,
		event_fn: impl FnOnce(Entity) -> E,
	) -> &mut Self {
		let event = (event_fn)(self.id());
		self.commands_mut().queue_silenced(command::trigger(event));
		self
	}

	/// Queues an asynchronous task for this entity.
	///
	/// The task receives an [`AsyncEntity`] handle and, being entity-scoped, ends
	/// cleanly if the entity is despawned while it runs (a long-lived reconnect or
	/// accept loop outliving a scene swap) rather than routing the resulting error
	/// to the panicking handler. If the entity is already gone by the time the
	/// command applies, the task is never spawned and the drop is logged at
	/// `debug` against the queueing call site.
	#[cfg(feature = "bevy_async")]
	#[track_caller]
	fn queue_async<Func, Fut, Out>(&mut self, func: Func)
	where
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		let id = self.id();
		let location = Location::caller();
		self.commands().queue(move |world: &mut World| {
			match world.get_entity_mut(id) {
				Ok(mut entity) => {
					entity.run_async(func);
				}
				Err(_) => debug!(
					"entity {id} despawned before its queued async task ran (at {location})"
				),
			}
		});
	}

	/// Queues a local asynchronous task for this entity, the `_local` (thread-bound
	/// `Fut`) sibling of [`queue_async`](Self::queue_async) with the same
	/// entity-scoped semantics: it ends cleanly if the entity is despawned while it
	/// runs, and is never spawned (a `debug` log against the queueing call site) if
	/// the entity is already gone when the command applies.
	#[cfg(feature = "bevy_async")]
	#[track_caller]
	fn queue_async_local<Func, Fut, Out>(&mut self, func: Func)
	where
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		let id = self.id();
		let location = Location::caller();
		self.commands().queue(move |world: &mut World| {
			match world.get_entity_mut(id) {
				Ok(mut entity) => {
					entity.run_async_local(func);
				}
				Err(_) => debug!(
					"entity {id} despawned before its queued async task ran (at {location})"
				),
			}
		});
	}
}
