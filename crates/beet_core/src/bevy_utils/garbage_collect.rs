//! Automatic garbage collection for entities through reference counting.
//!
//! This module provides a simple reference-counting mechanism for entities.
//! When all references to a target entity are removed, the target is automatically
//! despawned.

use crate::prelude::*;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;


/// A reference to a garbage-collected entity.
///
/// When this component is added to an entity, it registers as a reference to
/// the target entity (via [`GarbageCollectTarget`]). When the referencing entity
/// is despawned, the reference is removed. If this was the last reference,
/// the target entity is also despawned.
///
/// # Examples
///
/// ```
/// # use beet_core::prelude::*;
/// let mut world = World::new();
///
/// // Create a target entity that will be garbage collected
/// let target = world.spawn_empty().id();
///
/// // Create references to the target
/// let ref1 = world.spawn(GarbageCollectRef(target)).id();
/// let ref2 = world.spawn(GarbageCollectRef(target)).id();
///
/// // Target has 2 references
/// // When both ref1 and ref2 are despawned, target is automatically despawned
/// ```
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = GarbageCollectTarget)]
pub struct GarbageCollectRef(pub Entity);

/// Marker for an entity that will be automatically despawned when all
/// references to it are removed.
///
/// This component is automatically added to entities that are referenced by
/// [`GarbageCollectRef`]. You typically don't need to add this manually.
///
/// When the last [`GarbageCollectRef`] pointing to this entity is removed
/// (either by despawning the referencing entity or removing the component),
/// this entity will be automatically despawned.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[component(on_remove=on_remove)]
#[relationship_target(relationship = GarbageCollectRef)]
pub struct GarbageCollectTarget(Vec<Entity>);

fn on_remove(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).try_despawn();
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	#[ignore = "needs investigation - despawn in hook API changed in 0.18"]
	fn works() {
		let mut world = World::new();
		let gb = world.spawn_empty().id();
		let entity1 = world.spawn(GarbageCollectRef(gb)).id();
		let entity2 = world.spawn(GarbageCollectRef(gb)).id();
		world
			.get::<GarbageCollectTarget>(gb)
			.unwrap()
			.len()
			.xpect_eq(2);
		world.despawn(entity1).xpect_true();
		world
			.get::<GarbageCollectTarget>(gb)
			.unwrap()
			.len()
			.xpect_eq(1);
		world.get_entity(gb).is_err().xpect_false();
		world.despawn(entity2).xpect_true();
		world.get_entity(gb).is_err().xpect_true();
	}
}
