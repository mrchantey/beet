use crate::prelude::*;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;


/// Added to a 'watched' entity, despawning it
/// will remove the [`GarbageCollectTarget`] if its the last reference.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = GarbageCollectTarget)]
pub struct GarbageCollectRef(pub Entity);

/// Added to an entity that will be despawned when all
/// references to it are removed.
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
	use sweet::prelude::*;

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
