use crate::prelude::*;
use bevy::prelude::*;

impl EntityIdent {
	pub fn get_roots(world: &mut World) -> Vec<EntityIdent> {
		world
			.query_filtered::<Entity, With<BeetRoot>>()
			.iter(world)
			.map(|e| EntityIdent::new(e))
			.collect()
	}

	/// Add a node as a child of the given entity
	pub fn add_child_behavior(self, world: &mut World) -> EntityIdent {
		let entity = world.spawn(Name::new("New Node")).id();
		
		if let Some(mut parent) = world.get_entity_mut(*self) {
			parent.add_child(entity);
		} else {
			log::warn!("parent not found when adding node");
		}

		EntityIdent(entity)
	}
}


#[cfg(test)]
mod test {
	// use crate::prelude::*;
	// use bevy::prelude::*;

	// fn world() -> World {
	// 	let mut world = World::new();
	// 	world.init_resource::<AppTypeRegistry>();
	// 	let registry = world.resource_mut::<AppTypeRegistry>();
	// 	let mut registry = registry.write();
	// 	registry.register::<SetOnRun<RunResult>>();
	// 	registry.register::<BeetRoot>();
	// 	drop(registry);
	// 	world
	// }

	// fn node(world: &mut World) -> EntityIdent {
	// 	BeetBuilder::new(SetOnRun(RunResult::Success))
	// 		.build(world)
	// 		.node()
	// }
}
