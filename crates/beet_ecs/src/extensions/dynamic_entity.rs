use crate::prelude::*;
use bevy::prelude::*;
use bevy::reflect::FromReflect;
use bevy::scene::DynamicEntity;

#[extend::ext]
pub impl DynamicEntity {
	fn new(entity: Entity, world: &impl IntoWorld) -> Self {
		let world = world.into_world_ref();
		let components = reflect_entity(world, entity);
		Self { entity, components }
	}

	fn name(&self) -> String {
		for component in self.components.iter() {
			if let Some(name) =
				<Name as FromReflect>::from_reflect(component.as_ref())
			{
				return name.as_str().to_string();
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
	use petgraph::graph::DiGraph;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();
		app.register_type::<Name>();
		let entity = app.world.spawn(Name::new("Bob")).id();
		let entity = DynamicEntity::new(entity, &app);
		expect(entity.components.len()).to_be(1)?;
		expect(entity.name().as_str()).to_be("Bob")?;

		let mut digraph = DiGraph::new();
		digraph.add_node(entity.entity);
		let entity_graph = EntityGraph(digraph);
		let dynamic_graph =
			DynamicEntityGraph::from_entity_graph(&mut app, entity_graph);
		let root = dynamic_graph.root().unwrap();
		expect(root.name().as_str()).to_be("Bob")?;

		Ok(())
	}
}
