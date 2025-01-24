use anyhow::Result;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use bevy::scene::DynamicEntity;

#[extend::ext]
pub impl DynamicEntity {
	fn new(world: &World, entity: Entity) -> Result<Self>
	where
		Self: Sized,
	{
		if world.get_entity(entity).is_err() {
			anyhow::bail!("Entity not found: {}", entity);
		}
		let scene = DynamicSceneBuilder::from_world(world)
			.extract_entities(vec![entity].into_iter())
			.build();
		let out = scene.entities.into_iter().next().unwrap();
		Ok(out)
	}

	fn apply(self, world: &mut World) -> Result<()> {
		// in case components were removed, we need to clear the entity
		world.entity_mut(self.entity).retain::<()>();

		let mut entity_map = EntityHashMap::default();
		entity_map.insert(self.entity, self.entity);
		let scene = DynamicScene {
			resources: Default::default(),
			entities: vec![self],
		};
		scene.write_to_world(world, &mut entity_map)?;
		Ok(())
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use bevy::scene::DynamicEntity;
	use sweet::prelude::*;

	fn node_name(entity: &DynamicEntity) -> String {
		for component in entity.components.iter() {
			if let Some(name) =
				<Name as FromReflect>::from_reflect(component.as_ref())
			{
				return name.to_string();
			}
		}
		format!("New Entity {:?}", entity.entity)
	}

	#[test]
	fn works() {
		let mut app = App::new();
		app.register_type::<Name>();
		let entity_id = app.world_mut().spawn(Name::new("Bob")).id();
		let mut entity = DynamicEntity::new(app.world(), entity_id).unwrap();
		expect(entity.components.len()).to_be(1);
		let name = node_name(&entity);
		expect(name.as_str()).to_be("Bob");

		entity.components[0].apply(&Name::new("Alice"));

		expect(app.world())
			.component::<Name>(entity_id)
			.to_be(&Name::new("Bob"));
		entity.apply(app.world_mut()).unwrap();
		expect(app.world())
			.component::<Name>(entity_id)
			.to_be(&Name::new("Alice"));
	}
}
