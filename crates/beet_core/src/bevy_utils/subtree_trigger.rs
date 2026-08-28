//! Triggers that fire one event instance across a whole subtree.
//!
//! Where [`EntityTargetTrigger`] walks *up* a hierarchy one hop at a time,
//! [`SubtreeTrigger`] walks *down* a snapshot: every entity at or under a root,
//! deepest first and the root last, each exactly once, never above the root.
//! That is the shape a settle signal needs ([`Ready`](crate::prelude::Ready)):
//! a node observes its own event only after everything it owns has observed
//! theirs. [`ScopedTrigger`] is its opt-in sibling: it sweeps only where the
//! target declares [`StartDescendants`], firing on the target alone otherwise.
//! All are fired the same way, through `trigger_target`.

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
}

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
		let targets = snapshot(&world, self.root);
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

/// Opts an entity into sweeping its subtree when a [`ScopedTrigger`] event
/// targets it: with it the event fires on every descendant too (deepest first,
/// the entity last), without it the event fires on the entity alone.
///
/// Read by the *trigger* rather than an observer, so delivery scope is plain
/// entity data: no re-fire, and a nested declaring entity sweeps only on its
/// own starts. Wired by `#[require]` where a type's starts should always fan
/// out (a `RunningSet`), so hot single-target starts (`ContinueRun`) never pay
/// for a sweep they don't want.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct StartDescendants;

/// An [`EntityEvent`] [`Trigger`] whose delivery scope is declared on the
/// target: a target with [`StartDescendants`] receives a [`SubtreeTrigger`]
/// style sweep, any other target receives the event alone.
///
/// Fired like any other entity target event:
/// `entity.trigger_target(|entity| MyEvent { entity })`. The sweep semantics
/// (pre-observer snapshot, despawn skipping, per-entity retargeting) match
/// [`SubtreeTrigger`].
pub struct ScopedTrigger<E> {
	/// The original target: the sweep's origin when sweeping, the sole target
	/// otherwise.
	///
	/// [`Entity::PLACEHOLDER`] until the trigger fires, when it is read off the
	/// event's target.
	root: Entity,
	_marker: PhantomData<E>,
}

impl<E> Default for ScopedTrigger<E> {
	fn default() -> Self {
		Self {
			root: Entity::PLACEHOLDER,
			_marker: PhantomData,
		}
	}
}

impl<E> ScopedTrigger<E> {
	/// The event's original target, whichever entity a sweep landed it on.
	pub fn root(&self) -> Entity { self.root }
}

impl<E> fmt::Debug for ScopedTrigger<E> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ScopedTrigger")
			.field("root", &self.root)
			.finish()
	}
}

// SAFETY:
// - `E`'s [`Event::Trigger`] is constrained to [`ScopedTrigger<E>`]
unsafe impl<E> Trigger<E> for ScopedTrigger<E>
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
		self.root = event.event_target();
		// the target's own declaration decides the scope, snapshotted before the
		// first observer can restructure the tree.
		let targets = match world.get::<StartDescendants>(self.root).is_some() {
			true => snapshot(&world, self.root),
			false => vec![self.root],
		};
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

	#[derive(Debug, Clone)]
	struct Scoped {
		entity: Entity,
	}
	impl Event for Scoped {
		type Trigger<'a> = ScopedTrigger<Self>;
	}
	impl EntityEvent for Scoped {
		fn event_target(&self) -> Entity { self.entity }
	}
	impl SetEntityEventTarget for Scoped {
		fn set_event_target(&mut self, entity: Entity) { self.entity = entity; }
	}

	/// Record every entity a [`Scoped`] fire lands on.
	fn record(world: &mut World) -> Store<Vec<Entity>> {
		let fired = Store::new(Vec::<Entity>::new());
		let recorder = fired.clone();
		world.add_observer(move |ev: On<Scoped>| {
			let mut all = recorder.get();
			all.push(ev.entity);
			recorder.set(all);
		});
		fired
	}

	#[beet_core::test]
	fn a_declaring_target_sweeps() {
		let mut world = World::new();
		let [root, a, b, c, d] = tree(&mut world);
		world.entity_mut(root).insert(StartDescendants);
		let fired = record(&mut world);

		world
			.entity_mut(root)
			.trigger_target(|entity| Scoped { entity });

		fired.get().xpect_eq(vec![d, c, b, a, root]);
	}

	#[beet_core::test]
	fn a_bare_target_fires_alone() {
		let mut world = World::new();
		let [root, ..] = tree(&mut world);
		let fired = record(&mut world);

		world
			.entity_mut(root)
			.trigger_target(|entity| Scoped { entity });

		fired.get().xpect_eq(vec![root]);
	}

	/// A nested declaring entity sweeps only on its own starts: an outer sweep
	/// delivers to it once, never re-fanning through it.
	#[beet_core::test]
	fn a_nested_declaration_never_double_delivers() {
		let mut world = World::new();
		let [root, a, b, c, d] = tree(&mut world);
		world.entity_mut(root).insert(StartDescendants);
		world.entity_mut(a).insert(StartDescendants);
		let fired = record(&mut world);

		world
			.entity_mut(root)
			.trigger_target(|entity| Scoped { entity });
		fired.get().xpect_eq(vec![d, c, b, a, root]);

		// the nested declaration's own start sweeps its subtree alone.
		fired.set(Vec::new());
		world
			.entity_mut(a)
			.trigger_target(|entity| Scoped { entity });
		fired.get().xpect_eq(vec![d, c, a]);
	}
}
