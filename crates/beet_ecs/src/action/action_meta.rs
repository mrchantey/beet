use bevy::reflect::reflect_trait;
use crate::prelude::*;

#[reflect_trait]
pub trait ActionMeta {
	fn graph_role(&self) -> GraphRole;
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::reflect::ReflectFromPtr;
	use std::any::Any;
	use sweet::*;


	#[derive_action]
	#[action(graph_role = GraphRole::Node)]
	struct MyStruct;

	fn my_struct() {}

	#[test]
	fn works() -> Result<()> {
		let mut registry = TypeRegistry::default();
		MyStruct::register_types(&mut registry);

		let val = MyStruct;
		expect(val.graph_role()).to_be(GraphRole::Node)?;
		let data = registry
			.get_type_data::<ReflectActionMeta>(MyStruct.type_id())
			.unwrap();
		let val: &dyn ActionMeta = data.get(&val).unwrap();
		expect(val.graph_role()).to_be(GraphRole::Node)?;


		Ok(())
	}

	#[test]
	fn works_ptr() -> Result<()> {
		let mut world = World::new();
		world.init_resource::<AppTypeRegistry>();
		let mut registry = world.resource::<AppTypeRegistry>().write();
		registry.register::<MyStruct>();
		drop(registry);
		let entity = world.spawn(MyStruct).id();


		let registry = world.resource::<AppTypeRegistry>().read();
		let type_id = MyStruct.type_id();
		let registration = registry.get(type_id).unwrap();
		let component_id = world.components().get_id(type_id).unwrap();
		let entity = world.get_entity(entity).unwrap();
		let component = entity.get_by_id(component_id).unwrap();
		let component = unsafe {
			registration
				.data::<ReflectFromPtr>()
				.unwrap()
				.as_reflect(component)
		};
		let data = registry
			.get_type_data::<ReflectActionMeta>(type_id)
			.unwrap();
		let component: &dyn ActionMeta = data.get(component).unwrap();

		expect(component.graph_role()).to_be(GraphRole::Node)?;

		Ok(())
	}
}
