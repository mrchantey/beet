// how to fix this late bound?
#![allow(late_bound_lifetime_arguments)]
use crate::prelude::*;
use beet_core::prelude::*;

/// A scene-serializable technique for triggering an event on the entity
/// this component is attached to. On spawn this component is immediately
/// removed and replaced with an `OnSpawnDeferred` command to be executed on
/// the next `OnSpawnDeferred::flush`. This indirection allows this component to be
/// inserted before any listeners, and only trigger after they have been inserted.
///
/// ## Example
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// let mut world = World::new();
/// world.spawn((TriggerDeferred::run(), EndOnRun(SUCCESS)));
/// world.run_system_cached(OnSpawnDeferred::flush).unwrap();
/// ```
/// ## Notes
/// This component is SparsSet as it is frequently added and removed.
#[derive(Clone, Component)]
#[component(storage = "SparseSet",on_add=on_add::<T>)]
pub struct TriggerDeferred<T: EventPayload> {
	event: T,
}

impl<T> Default for TriggerDeferred<T>
where
	T: EventPayload + Default,
{
	fn default() -> Self { Self::new(default()) }
}

impl TriggerDeferred<RequestEndResult> {
	pub fn run() -> Self { default() }
}


impl<T: EventPayload> TriggerDeferred<T> {
	/// Create a new [`TriggerOnSpawn`] with the provided event
	pub fn new(event: T) -> Self { Self { event } }
}

fn on_add<T: EventPayload>(mut world: DeferredWorld, cx: HookContext) {
	let entity = cx.entity;
	world.commands().queue(move |world: &mut World| -> Result {
		let ev = world
			.entity_mut(entity)
			.take::<TriggerDeferred<T>>()
			.ok_or_else(|| bevyhow!("TriggerDeferred: component missing"))?;
		world
			.entity_mut(entity)
			.insert(OnSpawnDeferred::trigger(ev.event));
		Ok(())
	});
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();
		let observers = observer_ext::observe_triggers::<End>(&mut world);
		world.spawn((TriggerDeferred::run(), EndOnRun(SUCCESS)));
		observers.len().xpect_eq(0);
		world.run_system_cached(OnSpawnDeferred::flush).unwrap();
		observers.len().xpect_eq(1);
		observers.get_index(0).unwrap().value().xpect_eq(SUCCESS);
	}
}
