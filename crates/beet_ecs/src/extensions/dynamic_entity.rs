use crate::prelude::*;
use bevy::prelude::*;
use bevy::reflect::FromReflect;
use bevy::scene::DynamicEntity;

#[extend::ext]
pub impl DynamicEntity {
	// fn new(entity: Entity, world: &impl IntoWorld) -> Self {
	// 	let world = world.into_world_ref();
	// 	let components = reflect_entity(world, entity);
	// 	Self { entity, components }
	// }
	fn new(entity: Entity, world: &World) -> Self {
		let scene = DynamicSceneBuilder::from_world(world)
			.extract_entities(vec![entity].into_iter())
			.build();
		scene.entities.into_iter().next().unwrap()
	}

	fn node_name(&self) -> String {
		for component in self.components.iter() {
			if let Some(name) =
				<NodeName as FromReflect>::from_reflect(component.as_ref())
			{
				return name.to_string();
			}
		}
		format!("New Entity {:?}", self.entity)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use bevy::scene::DynamicEntity;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();
		app.register_type::<NodeName>();
		let entity = app.world.spawn(NodeName::new("Bob")).id();
		let entity = DynamicEntity::new(entity, &app.world);
		expect(entity.components.len()).to_be(1)?;
		expect(entity.node_name().as_str()).to_be("Bob")?;
		Ok(())
	}
}
