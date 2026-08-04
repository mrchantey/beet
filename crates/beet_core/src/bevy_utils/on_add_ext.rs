//! Component hook constructors for common `on_add` patterns.
//!
//! Bevy's `#[component(on_add = ...)]` attribute accepts a function call
//! yielding a closure, evaluated fresh on every hook invocation. These
//! constructors exploit that to express common hooks inline:
//!
//! ```
//! # use beet_core::prelude::*;
//! #[derive(EntityTargetEvent)]
//! struct MyEvent;
//!
//! fn log_event(ev: On<MyEvent>) {}
//!
//! #[derive(Component)]
//! #[component(on_add = on_add_ext::observe(log_event))]
//! struct Watched;
//!
//! #[derive(Component)]
//! #[component(on_add = on_add_ext::entity_hook(|entity| { entity.insert(Name::new("hooked")); }))]
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
/// Prefer this over a bespoke `fn on_add(world: DeferredWorld, cx: HookContext)`
/// whenever the hook only queues entity commands.
pub fn entity_hook(
	func: impl FnOnce(&mut EntityCommands),
) -> impl FnOnce(DeferredWorld, HookContext) {
	move |mut world: DeferredWorld, cx: HookContext| {
		func(&mut world.commands().entity(cx.entity));
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
	#[component(on_add = on_add_ext::observe(increment))]
	struct Single;

	#[derive(Component)]
	#[component(on_add = on_add_ext::observe((increment, increment)))]
	struct Pair;

	#[derive(Component)]
	#[component(on_add = on_add_ext::entity_hook(|entity| { entity.insert(Name::new("hooked")); }))]
	struct Named;

	#[beet_core::test]
	fn observe_single() {
		let mut world = World::new();
		world.init_resource::<Count>();
		let entity = world.spawn(Single).id();
		world.flush();
		world.entity_mut(entity).trigger_target(Ping);
		world.resource::<Count>().0.xpect_eq(1);
	}

	#[beet_core::test]
	fn observe_tuple() {
		let mut world = World::new();
		world.init_resource::<Count>();
		let entity = world.spawn(Pair).id();
		world.flush();
		world.entity_mut(entity).trigger_target(Ping);
		world.resource::<Count>().0.xpect_eq(2);
	}

	#[beet_core::test]
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
}
