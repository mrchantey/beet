//! A [`Trigger`] that fires one event instance across a whole subtree.
//!
//! Where [`EntityTargetTrigger`] walks *up* a hierarchy one hop at a time,
//! [`SubtreeTrigger`] walks *down* a snapshot: every entity at or under a root,
//! deepest first and the root last, each exactly once, never above the root.
//! That is the shape a settle signal needs ([`Ready`](crate::prelude::Ready)):
//! a node observes its own event only after everything it owns has observed
//! theirs. Both are fired the same way, through `trigger_target`.

use crate::prelude::*;
use bevy::ecs::event::SetEntityEventTarget;
use bevy::ecs::event::Trigger;
use bevy::ecs::event::trigger_entity_internal;
use bevy::ecs::observer::CachedObservers;
use bevy::ecs::observer::TriggerContext;
use core::fmt;

/// An [`EntityEvent`] [`Trigger`] that fires one event instance on every entity
/// of a subtree, deepest first and the root last.
///
/// The root is the event's target, so a sweep is fired like any other entity
/// target event: `entity.trigger_target(|entity| MyEvent { entity })`.
///
/// The subtree is snapshotted before the first observer runs, so an observer
/// that restructures or despawns part of the tree can neither redirect the
/// sweep nor be visited twice; a despawned target is skipped. The event's
/// target is retargeted per entity through [`SetEntityEventTarget`], so an
/// observer reads the entity it fired on.
pub struct SubtreeTrigger<E> {
	/// The subtree root: the sweep's origin, and the last entity it fires on.
	///
	/// [`Entity::PLACEHOLDER`] until the sweep starts, when it is read off the
	/// event's target.
	root: Entity,
	_marker: PhantomData<E>,
}

impl<E> Default for SubtreeTrigger<E> {
	fn default() -> Self {
		Self {
			root: Entity::PLACEHOLDER,
			_marker: PhantomData,
		}
	}
}

impl<E> SubtreeTrigger<E> {
	/// The subtree root the sweep originated at, ie the event's original target.
	pub fn root(&self) -> Entity { self.root }

	/// `root`'s subtree read through [`Children`], deepest first and `root` last.
	fn snapshot(world: &World, root: Entity) -> Vec<Entity> {
		let mut targets = vec![root];
		let mut index = 0;
		// breadth-first, so reversing settles every child before its parent.
		while index < targets.len() {
			if let Some(children) = world.get::<Children>(targets[index]) {
				targets.extend(children.iter());
			}
			index += 1;
		}
		targets.reverse();
		targets
	}
}

impl<E> fmt::Debug for SubtreeTrigger<E> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("SubtreeTrigger")
			.field("root", &self.root)
			.finish()
	}
}

// SAFETY:
// - `E`'s [`Event::Trigger`] is constrained to [`SubtreeTrigger<E>`]
unsafe impl<E> Trigger<E> for SubtreeTrigger<E>
where
	E: SetEntityEventTarget + for<'a> Event<Trigger<'a> = Self>,
{
	unsafe fn trigger(
		&mut self,
		mut world: DeferredWorld,
		observers: &CachedObservers,
		trigger_context: &TriggerContext,
		event: &mut E,
	) {
		// the sweep starts where the event was targeted, and snapshots before the
		// first observer can restructure the tree.
		self.root = event.event_target();
		let targets = Self::snapshot(&world, self.root);
		for target in targets {
			// an earlier observer may have despawned this entity.
			if world.get_entity(target).is_err() {
				continue;
			}
			event.set_event_target(target);
			// SAFETY:
			// - `observers` come from `world` and match the event type `E`, enforced by the call to `trigger`
			// - the passed in event pointer comes from `event`, which is an `Event`
			// - `trigger` is a matching trigger type, as it comes from `self`, which is the Trigger for `E`
			// - `trigger_context`'s event_key matches `E`, enforced by the call to `trigger`
			unsafe {
				trigger_entity_internal(
					world.reborrow(),
					observers,
					event.into(),
					self.into(),
					target,
					trigger_context,
				);
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::ecs::event::SetEntityEventTarget;

	#[derive(Debug, Clone)]
	struct Swept {
		entity: Entity,
	}
	impl Event for Swept {
		type Trigger<'a> = SubtreeTrigger<Self>;
	}
	impl EntityEvent for Swept {
		fn event_target(&self) -> Entity { self.entity }
	}
	impl SetEntityEventTarget for Swept {
		fn set_event_target(&mut self, entity: Entity) { self.entity = entity; }
	}

	/// tree: root -> [a -> [c, d], b]
	fn tree(world: &mut World) -> [Entity; 5] {
		let a = world.spawn_empty().id();
		let b = world.spawn_empty().id();
		let c = world.spawn_empty().id();
		let d = world.spawn_empty().id();
		world.entity_mut(a).add_children(&[c, d]);
		let root = world.spawn_empty().add_children(&[a, b]).id();
		[root, a, b, c, d]
	}

	#[beet_core::test]
	fn fires_bottom_up_exactly_once() {
		let mut world = World::new();
		let [root, a, b, c, d] = tree(&mut world);
		let fired = Store::new(Vec::<Entity>::new());
		let recorder = fired.clone();
		// every fire reports the sweep's origin, whichever entity it landed on
		let origins = Store::new(HashSet::<Entity>::default());
		let seen_origins = origins.clone();
		world.add_observer(move |ev: On<Swept>| {
			let mut all = recorder.get();
			all.push(ev.entity);
			recorder.set(all);
			let mut roots = seen_origins.get();
			roots.insert(ev.trigger().root());
			seen_origins.set(roots);
		});

		world
			.entity_mut(root)
			.trigger_target(|entity| Swept { entity });

		fired.get().xpect_eq(vec![d, c, b, a, root]);
		origins.get().xpect_eq(HashSet::from_iter([root]));
	}

	/// a sweep is an entity target event like any other, so it also queues.
	#[beet_core::test]
	fn fires_from_commands() {
		let mut world = World::new();
		let [root, a, b, c, d] = tree(&mut world);
		let fired = Store::new(Vec::<Entity>::new());
		let recorder = fired.clone();
		world.add_observer(move |ev: On<Swept>| {
			let mut all = recorder.get();
			all.push(ev.entity);
			recorder.set(all);
		});

		world
			.commands()
			.entity(root)
			.trigger_target(|entity| Swept { entity });
		world.flush();

		fired.get().xpect_eq(vec![d, c, b, a, root]);
	}

	#[beet_core::test]
	fn never_fires_above_the_root() {
		let mut world = World::new();
		let [root, a, _b, c, d] = tree(&mut world);
		let fired = Store::new(Vec::<Entity>::new());
		let recorder = fired.clone();
		world.add_observer(move |ev: On<Swept>| {
			let mut all = recorder.get();
			all.push(ev.entity);
			recorder.set(all);
		});

		// sweeping a mid-tree node reaches its own subtree and stops there.
		world
			.entity_mut(a)
			.trigger_target(|entity| Swept { entity });

		let fired = fired.get();
		fired.contains(&root).xpect_false();
		fired.xpect_eq(vec![d, c, a]);
	}
}
