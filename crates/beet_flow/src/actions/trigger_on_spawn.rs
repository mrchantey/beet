// how to fix this late bound?
#![allow(late_bound_lifetime_arguments)]
use crate::prelude::*;
use beet_core::prelude::*;


pub fn run_on_spawn() -> TriggerOnSpawn<'static, false, Run, &'static ChildOf> {
	default()
}

/// Sometimes its useful to run an action by spawning an entity,
/// for example if you want to run on the next frame to avoid
/// infinite loops or await updated world state.
/// The [`RunOnSpawn`] component will be removed immediately
/// and the [`OnRunAction`] will be triggered.
/// ## Example
/// ```
/// # use beet_flow::doctest::*;
/// # let mut world = world();
/// world.spawn(RunOnSpawn::default());
/// ```
/// ## Notes
/// This component is SparsSet as it is frequently added and removed.
#[derive(Debug, Clone, Component)]
#[component(storage = "SparseSet",on_add=on_add::<'a,AUTO_PROPAGATE,E,T>)]
pub struct TriggerOnSpawn<
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
> Default for TriggerOnSpawn<'a, AUTO_PROPAGATE, E, T>
{
	fn default() -> Self { Self::new(E::default()) }
}

impl<
	'a,
	const AUTO_PROPAGATE: bool,
	E: Event<Trigger<'a> = EntityTargetTrigger<AUTO_PROPAGATE, E, T>>,
	T: 'static + Send + Sync + Traversal<E>,
> TriggerOnSpawn<'a, AUTO_PROPAGATE, E, T>
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
			.take::<TriggerOnSpawn<'a, AUTO_PROPAGATE, E, T>>()
			.expect("TriggerOnSpawn: component missing");
		world.entity_mut(entity).trigger_target(ev.event);
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
		world.spawn((EndOnRun::success(), run_on_spawn()));
		observers.len().xpect_eq(1);
		observers.get_index(0).unwrap().xpect_eq(SUCCESS);
		// app.world_mut().flush();
	}
}
