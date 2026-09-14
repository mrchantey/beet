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

	/// Raises `err` through the world's configured error handler, the
	/// [`Commands`] sibling of [`World::handle_command_error`](crate::prelude::WorldExt::handle_command_error).
	///
	/// This is how code outside a system raises: a component hook reaches it via
	/// [`DeferredWorld::commands`](bevy::ecs::world::DeferredWorld::commands),
	/// never `panic!`, `debug_assert!` or a bare `error!`.
	#[track_caller]
	fn handle_command_error<F>(&mut self, err: BevyError) {
		self.handle_command_error_with_location::<F>(err, Location::caller());
	}

	/// Like [`handle_command_error`](Self::handle_command_error), but attributed
	/// to an explicit `location`, for a raise deferred past its call site.
	fn handle_command_error_with_location<F>(
		&mut self,
		err: BevyError,
		location: &'static Location<'static>,
	) {
		self.queue(move |world: &mut World| {
			world.handle_command_error_with_location::<F>(err, location);
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
	/// The task receives an [`AsyncEntity`] handle and is entity-scoped: it is
	/// cancelled if the entity is despawned while it runs (a reconnect or accept
	/// loop ends with its scene). If the entity is already gone by the time the
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
		queue_if_alive(self, Location::caller(), move |mut entity| {
			entity.run_async(func);
		});
	}

	/// Queues a local asynchronous task for this entity, the `_local` (thread-bound
	/// `Fut`) sibling of [`queue_async`](Self::queue_async) with the same
	/// entity-scoped semantics: it is cancelled if the entity is despawned while
	/// it runs, and is never spawned (a `debug` log against the queueing call
	/// site) if the entity is already gone when the command applies.
	#[cfg(feature = "bevy_async")]
	#[track_caller]
	fn queue_async_local<Func, Fut, Out>(&mut self, func: Func)
	where
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		queue_if_alive(self, Location::caller(), move |mut entity| {
			entity.run_async_local(func);
		});
	}

	/// Queues an asynchronous task for this entity that one of its components
	/// owns. The spawn happens when the command applies, so the [`AsyncTask`]
	/// is handed to `own` then and the bundle it returns is inserted on the
	/// entity: a tuple-struct constructor is the usual `own`
	/// (`queue_task(ReloadTail, func)`), and the task is cancelled when that
	/// component is removed, replaced by a later insert, or despawned with the
	/// entity.
	///
	/// Never spawned (a `debug` log against the queueing call site) if the
	/// entity is already gone when the command applies, as
	/// [`queue_async`](Self::queue_async) is.
	#[cfg(feature = "bevy_async")]
	#[track_caller]
	fn queue_task<B, Func, Fut, Out>(
		&mut self,
		own: impl 'static + Send + FnOnce(AsyncTask) -> B,
		func: Func,
	) where
		B: Bundle,
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		queue_if_alive(self, Location::caller(), move |mut entity| {
			let task = entity.run_task(func);
			entity.insert(own(task));
		});
	}

	/// Queues a local asynchronous task for this entity that one of its
	/// components owns, the `_local` sibling of [`queue_task`](Self::queue_task).
	#[cfg(feature = "bevy_async")]
	#[track_caller]
	fn queue_task_local<B, Func, Fut, Out>(
		&mut self,
		own: impl 'static + Send + FnOnce(AsyncTask) -> B,
		func: Func,
	) where
		B: Bundle,
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		queue_if_alive(self, Location::caller(), move |mut entity| {
			let task = entity.run_task_local(func);
			entity.insert(own(task));
		});
	}
}

/// Queue `func` against the entity, skipping it with a `debug` log against
/// `location` if the entity has been despawned by the time the command applies.
#[cfg(feature = "bevy_async")]
fn queue_if_alive(
	commands: &mut EntityCommands,
	location: &'static Location<'static>,
	func: impl 'static + Send + FnOnce(EntityWorldMut),
) {
	let id = commands.id();
	commands.commands().queue(move |world: &mut World| {
		match world.get_entity_mut(id) {
			Ok(entity) => func(entity),
			Err(_) => debug!(
				"entity {id} despawned before its queued async task ran (at {location})"
			),
		}
	});
}
