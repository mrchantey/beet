//! The action calls in flight on an entity, held apart from its other tasks.
use crate::prelude::*;
use beet_core::prelude::*;
use core::any::TypeId;
use core::pin::pin;
use core::task::Poll;

/// The action calls in flight on an entity, kept apart from its other tasks
/// ([`EntityTasks`], ie a server's connections) so either can be stopped
/// alone.
///
/// Each async call is its own entry, a task cancelled when the entry drops, so
/// a despawn ends every call with the entity and a caller dropping its future
/// ends its own call alone (see [`CallerGone`]). A parked call is the entity's
/// [`Running`], registered here by its output type so [`InterruptRun`]
/// resolves it whatever that type is. Finished entries are pruned on each
/// push, as [`EntityTasks`] prunes its own.
#[derive(Debug, Default, Component)]
#[component(clone_behavior = Ignore)]
pub struct ActionCalls {
	/// One per async call, cancelled on drop.
	tasks: Vec<AsyncTask>,
	/// How to interrupt each kind of [`Running`] the entity has parked.
	parked: Vec<(TypeId, fn(&mut World, Entity) -> Result)>,
}

impl ActionCalls {
	/// The async calls still in flight. A parked [`Running`] is not counted,
	/// being a component of its own.
	pub fn len(&self) -> usize {
		self.tasks.iter().filter(|task| !task.is_finished()).count()
	}

	/// Whether no async call is in flight.
	pub fn is_empty(&self) -> bool { self.len() == 0 }

	/// Stop every call in flight on `entity`: each async call ends at its next
	/// await and each parked [`Running`] resolves, both reaching their callers
	/// as [`ControlFlowError::Interrupted`].
	///
	/// # Errors
	/// Propagates an error from a parked call's [`OutHandler`].
	pub fn interrupt(world: &mut World, entity: Entity) -> Result {
		let Some(mut calls) = world.get_mut::<Self>(entity) else {
			return Ok(());
		};
		// dropping the handles cancels the tasks
		calls.tasks.clear();
		let parked = calls.parked.clone();
		for (_, interrupt) in parked {
			interrupt(world, entity)?;
		}
		Ok(())
	}

	/// Spawn one async call of `entity` through `spawn` and hold its task.
	/// When the command applies to an entity already gone the call is never
	/// spawned, its dropped [`OutHandler`] resolving the caller as interrupted.
	pub(crate) fn queue(
		commands: &mut Commands,
		entity: Entity,
		spawn: impl 'static + Send + FnOnce(&mut EntityWorldMut) -> AsyncTask,
	) {
		commands.queue(move |world: &mut World| {
			let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
				debug!("entity {entity} despawned before its action call ran");
				return;
			};
			let task = spawn(&mut entity_mut);
			match entity_mut.get_mut::<Self>() {
				Some(mut calls) => {
					calls.tasks.retain(|task| !task.is_finished());
					calls.tasks.push(task);
				}
				None => {
					entity_mut.insert(Self {
						tasks: vec![task],
						parked: default(),
					});
				}
			}
		});
	}

	/// One call's work: run `fut` and deliver its output to `out_handler`,
	/// unless the caller leaves first, which ends it at its next await.
	pub(crate) async fn run<Out>(
		world: AsyncWorld,
		fut: impl Future<Output: IntoResult<Out>>,
		out_handler: OutHandler<Out>,
	) -> Result
	where
		Out: 'static + Send,
	{
		let mut gone = pin!(out_handler.caller_gone().wait());
		let mut call = pin!(async move {
			let result = fut.await.into_result();
			out_handler.call_async(world, result).await
		});
		core::future::poll_fn(move |cx| {
			// the caller leaving is checked first, so a stopped call never resumes
			if gone.as_mut().poll(cx).is_ready() {
				return Poll::Ready(Ok(()));
			}
			call.as_mut().poll(cx)
		})
		.await
	}

	/// The `on_add` hook of [`Running<T>`], registering how to interrupt it.
	pub(crate) fn register_running<T: 'static + Send + Sync>(
		mut world: DeferredWorld,
		cx: HookContext,
	) {
		let Some(mut calls) = world.get_mut::<Self>(cx.entity) else {
			return;
		};
		let id = TypeId::of::<T>();
		if !calls.parked.iter().any(|(parked, _)| *parked == id) {
			calls.parked.push((id, interrupt_running::<T>));
		}
	}
}

/// Resolve `entity`'s [`Running<T>`] as interrupted, if it still holds one.
fn interrupt_running<T: 'static + Send + Sync>(
	world: &mut World,
	entity: Entity,
) -> Result {
	match world
		.get_entity_mut(entity)
		.ok()
		.and_then(|mut entity| entity.take::<Running<T>>())
	{
		Some(running) => running.interrupt(world),
		None => Ok(()),
	}
}
