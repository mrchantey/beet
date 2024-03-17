use crate::prelude::EntityGraph;
use bevy::prelude::*;
use bevy::reflect::ReflectFromPtr;
use bevy::scene::DynamicEntity;
use petgraph::graph::DiGraph;




pub fn reflect_entity(world: &World, entity: Entity) -> Vec<Box<dyn Reflect>> {
	let type_registry = world.resource::<AppTypeRegistry>().read();

	world
		.inspect_entity(entity)
		.into_iter()
		.filter_map(|info| info.type_id().map(|id| (info, id)))
		.filter_map(|(info, id)| type_registry.get(id).map(|reg| (info, reg)))
		.map(|(info, reg)| {
			let value = world.entity(entity).get_by_id(info.id()).unwrap();
			let value = unsafe {
				reg.data::<ReflectFromPtr>().unwrap().as_reflect(value)
			};
			value.clone_value()
		})
		.collect::<Vec<_>>()
}

pub fn reflect_graph(
	world: &World,
	root: Entity,
) -> DiGraph<DynamicEntity, ()> {
	let entity_graph = EntityGraph::from_world(world, root);
	entity_graph.map(
		|_, entity| DynamicEntity {
			components: reflect_entity(world, *entity),
			entity: *entity,
		},
		|_, _| (),
	)
}


#[cfg(test)]
mod test {
	use super::*;
	use anyhow::Result;
	use bevy::reflect::ReflectRef;
	use sweet::*;

	#[derive(Debug, PartialEq, Component, Reflect)]
	struct MyStruct(pub i32);

	#[test]
	fn works() -> Result<()> {
		pretty_env_logger::init();
		let mut app = App::new();

		let entity = app.world.spawn(MyStruct(7)).id();
		app.world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<MyStruct>();

		let reflects = reflect_entity(&app.world, entity);

		expect(reflects.len()).to_be(1)?;
		let reflect = &reflects[0];

		let ReflectRef::TupleStruct(reflect_struct) = reflect.reflect_ref()
		else {
			anyhow::bail!("not a struct")
		};

		let field = reflect_struct.iter_fields().next().unwrap();
		expect(&format!("{field:?}").as_str()).to_be(&"7")?;


		Ok(())
	}
}
