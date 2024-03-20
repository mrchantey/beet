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
		if world.get_entity(entity).is_none() {
			anyhow::bail!("Entity not found");
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
	use anyhow::Result;
	use bevy::prelude::*;
	use bevy::scene::DynamicEntity;
	use sweet::*;

	fn node_name(entity: &DynamicEntity) -> String {
		for component in entity.components.iter() {
			if let Some(name) =
				<NodeName as FromReflect>::from_reflect(component.as_ref())
			{
				return name.to_string();
			}
		}
		format!("New Entity {:?}", entity.entity)
	}

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();
		app.register_type::<NodeName>();
		let entity_id = app.world.spawn(NodeName::new("Bob")).id();
		let mut entity = DynamicEntity::new(&app.world, entity_id)?;
		expect(entity.components.len()).to_be(1)?;
		let name = node_name(&entity);
		expect(name.as_str()).to_be("Bob")?;

		entity.components[0].apply(&NodeName::new("Alice"));

		expect(&app.world)
			.component(entity_id)?
			.to_be(&NodeName::new("Bob"))?;
		entity.apply(&mut app.world)?;
		expect(&app.world)
			.component(entity_id)?
			.to_be(&NodeName::new("Alice"))?;


		Ok(())
	}
}
