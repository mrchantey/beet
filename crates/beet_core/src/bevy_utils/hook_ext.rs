//! Constructors for common component hooks.
//!
//! Bevy's hook attributes (`on_add`, `on_insert`, `on_replace`, `on_remove`,
//! `on_despawn`) accept a function call yielding a closure, evaluated fresh on
//! every hook invocation. These constructors exploit that to express common
//! hooks inline, and apply to any of the five:
//!
//! ```
//! # use beet_core::prelude::*;
//! #[derive(EntityTargetEvent)]
//! struct MyEvent;
//!
//! fn log_event(ev: On<MyEvent>) {}
//!
//! #[derive(Component)]
//! #[component(on_add = hook_ext::observe(log_event))]
//! struct Watched;
//!
//! #[derive(Component)]
//! #[component(on_add = hook_ext::entity_hook(|entity| { entity.insert(Name::new("hooked")); }))]
//! struct Named;
//! ```
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::system::IntoObserverSystem;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

use crate::prelude::EntityCommandsActionEventExt as _;

/// Creates a component hook from a function receiving the hooked entity's
/// [`EntityCommands`].
///
/// Prefer this over a bespoke `fn hook(world: DeferredWorld, cx: HookContext)`
/// whenever the hook only queues entity commands.
pub fn entity_hook(
	func: impl FnOnce(&mut EntityCommands),
) -> impl FnOnce(DeferredWorld, HookContext) {
	move |mut world: DeferredWorld, cx: HookContext| {
		func(&mut world.commands().entity(cx.entity));
	}
}

/// Creates a component hook from a function receiving the hooked component and
/// returning the work to queue on its entity.
///
/// The read-my-own-config shape: a hook capturing a declared field at hook time
/// (a server's `default_boot`) then queuing the work that uses it. Two steps
/// rather than one closure because the component borrow must be released before
/// the entity's [`EntityCommands`] can be taken.
///
/// The queue outlives the component borrow, so an `-> impl FnOnce(&mut
/// EntityCommands)` provider spells `+ use<>`: it captures the read fields by
/// value, never the `&self` it read them from.
pub fn component_hook<C, Queue>(
	func: impl FnOnce(&C) -> Queue,
) -> impl FnOnce(DeferredWorld, HookContext)
where
	C: Component,
	Queue: FnOnce(&mut EntityCommands),
{
	move |mut world: DeferredWorld, cx: HookContext| {
		// present by definition on `on_add`/`on_insert`; absent only on a removal
		// hook, where there is nothing left to read.
		let Some(queue) = world.get::<C>(cx.entity).map(func) else {
			return;
		};
		queue(&mut world.commands().entity(cx.entity));
	}
}

/// Creates a component hook registering an observer, or tuple of observers,
/// each watching the hooked entity.
///
/// Uses [`observe_any`](crate::prelude::EntityCommandsActionEventExt::observe_any),
/// so any [`Event`] type is accepted, not just [`EntityEvent`].
pub fn observe<M>(
	observers: impl HookObservers<M>,
) -> impl FnOnce(DeferredWorld, HookContext) {
	entity_hook(move |entity| observers.add_observers(entity))
}

/// An observer, or tuple of observers, registrable by the [`observe`] hook.
pub trait HookObservers<M> {
	/// Register each observer, watching `entity`.
	fn add_observers(self, entity: &mut EntityCommands);
}

impl<E: Event, B: Bundle, M, O: IntoObserverSystem<E, B, M>>
	HookObservers<fn(E, B, M)> for O
{
	fn add_observers(self, entity: &mut EntityCommands) {
		entity.observe_any(self);
	}
}

macro_rules! impl_hook_observers_tuple {
	($(($obs:ident, $marker:ident)),*) => {
		impl<$($obs, $marker,)*> HookObservers<($($marker,)*)> for ($($obs,)*)
		where
			$($obs: HookObservers<$marker>,)*
		{
			fn add_observers(self, entity: &mut EntityCommands) {
				#[allow(non_snake_case)]
				let ($($obs,)*) = self;
				$($obs.add_observers(entity);)*
			}
		}
	};
}

impl_hook_observers_tuple!((O1, M1), (O2, M2));
impl_hook_observers_tuple!((O1, M1), (O2, M2), (O3, M3));
impl_hook_observers_tuple!((O1, M1), (O2, M2), (O3, M3), (O4, M4));

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[derive(EntityTargetEvent)]
	struct Ping;

	#[derive(Default, Resource)]
	struct Count(u32);

	fn increment(_ev: On<Ping>, mut count: ResMut<Count>) { count.0 += 1; }

	#[derive(Component)]
	#[component(on_add = hook_ext::observe(increment))]
	struct Single;

	#[derive(Component)]
	#[component(on_add = hook_ext::observe((increment, increment)))]
	struct Pair;

	#[derive(Component)]
	#[component(on_add = hook_ext::entity_hook(|entity| { entity.insert(Name::new("hooked")); }))]
	struct Named;

	#[derive(Component)]
	#[component(on_add = hook_ext::component_hook(|config: &Configured| {
		let label = config.0;
		move |entity: &mut EntityCommands| { entity.insert(Name::new(label)); }
	}))]
	struct Configured(&'static str);

	#[crate::test]
	fn observe_single() {
		let mut world = World::new();
		world.init_resource::<Count>();
		let entity = world.spawn(Single).id();
		world.flush();
		world.entity_mut(entity).trigger_target(Ping);
		world.resource::<Count>().0.xpect_eq(1);
	}

	#[crate::test]
	fn observe_tuple() {
		let mut world = World::new();
		world.init_resource::<Count>();
		let entity = world.spawn(Pair).id();
		world.flush();
		world.entity_mut(entity).trigger_target(Ping);
		world.resource::<Count>().0.xpect_eq(2);
	}

	#[crate::test]
	fn entity_hook_inserts() {
		let mut world = World::new();
		let entity = world.spawn(Named).id();
		world.flush();
		world
			.entity(entity)
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("hooked");
	}

	/// The queued work reads the declared field, so a hook captures its own
	/// config rather than re-resolving it later.
	#[crate::test]
	fn component_hook_reads_its_config() {
		let mut world = World::new();
		let entity = world.spawn(Configured("declared")).id();
		world.flush();
		world
			.entity(entity)
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("declared");
	}
}
