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
/// world.spawn((TriggerDeferred::run(), EndOnRun::success()));
/// world.run_system_cached(OnSpawnDeferred::flush).unwrap();
/// ```
/// ## Notes
/// This component is SparsSet as it is frequently added and removed.
#[derive(Debug, Clone, Component)]
#[component(storage = "SparseSet",on_add=on_add::<'a,AUTO_PROPAGATE,E,T>)]
pub struct TriggerDeferred<
	'a,
	const AUTO_PROPAGATE: bool,
	E: Event<Trigger<'a> = EntityTargetTrigger<AUTO_PROPAGATE, E, T>>,
	T: 'static + Send + Sync + Traversal<E>,
> {
	event: E,
	phantom: std::marker::PhantomData<&'a T>,
}

impl<
	'a,
	const AUTO_PROPAGATE: bool,
	E: Default + Event<Trigger<'a> = EntityTargetTrigger<AUTO_PROPAGATE, E, T>>,
	T: 'static + Send + Sync + Traversal<E>,
> Default for TriggerDeferred<'a, AUTO_PROPAGATE, E, T>
{
	fn default() -> Self { Self::new(E::default()) }
}

impl TriggerDeferred<'static, false, Run, &'static ChildOf> {
	pub fn run() -> Self { default() }
}


impl<
	'a,
	const AUTO_PROPAGATE: bool,
	E: Event<Trigger<'a> = EntityTargetTrigger<AUTO_PROPAGATE, E, T>>,
	T: 'static + Send + Sync + Traversal<E>,
> TriggerDeferred<'a, AUTO_PROPAGATE, E, T>
{
	/// Create a new [`TriggerOnSpawn`] with the provided event
	pub fn new(ev: E) -> Self {
		Self {
			event: ev,
			phantom: default(),
		}
	}
}

fn on_add<
	'a,
	const AUTO_PROPAGATE: bool,
	E: Event<Trigger<'a> = EntityTargetTrigger<AUTO_PROPAGATE, E, T>>,
	T: 'static + Send + Sync + Traversal<E>,
>(
	mut world: DeferredWorld,
	cx: HookContext,
) where
	'a: 'static,
{
	let entity = cx.entity;
	world.commands().queue(move |world: &mut World| {
		let ev = world
			.entity_mut(entity)
			.take::<TriggerDeferred<'a, AUTO_PROPAGATE, E, T>>()
			.expect("TriggerDeferred: component missing");
		world
			.entity_mut(entity)
			.insert(OnSpawnDeferred::trigger(ev.event));
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
		world.spawn((TriggerDeferred::run(), EndOnRun::success()));
		observers.len().xpect_eq(0);
		world.run_system_cached(OnSpawnDeferred::flush).unwrap();
		observers.len().xpect_eq(1);
		observers.get_index(0).unwrap().xpect_eq(SUCCESS);
	}
}
