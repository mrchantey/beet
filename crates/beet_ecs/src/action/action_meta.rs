use crate::prelude::*;
use bevy::prelude::*;

/// Provide extra info for action components, useful for debugging, visualization etc.
#[reflect_trait]
pub trait ActionMeta {
	fn graph_role(&self) -> GraphRole { GraphRole::Node }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::ecs::schedule::SystemConfigs;
	use bevy::prelude::*;
	use bevy::reflect::ReflectFromPtr;
	use bevy::reflect::TypeRegistry;
	use std::any::Any;
	use sweet::*;

	#[derive(Component, Reflect)]
	#[reflect(ActionMeta)]
	struct MyStruct;

	impl ActionMeta for MyStruct {
		fn graph_role(&self) -> GraphRole { GraphRole::Node }
	}

	impl ActionSystems for MyStruct {
		fn systems() -> SystemConfigs { my_struct.in_set(TickSet) }
	}
	// impl ActionMeta for MyStruct {
	// 	fn graph_role(&self) -> GraphRole { GraphRole::Node }
	// }

	fn my_struct() {}

	#[test]
	fn works() -> Result<()> {
		let mut registry = TypeRegistry::default();
		registry.register::<MyStruct>();

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
