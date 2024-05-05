use crate::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::utils::intern::Interned;

#[derive(Resource, Clone)]
pub struct BeetConfig {
	pub schedule: Interned<dyn ScheduleLabel>,
}

impl Default for BeetConfig {
	fn default() -> Self { Self::new(Update) }
}


impl BeetConfig {
	pub fn new(schedule: impl ScheduleLabel) -> Self {
		Self {
			schedule: schedule.intern(),
		}
	}
}

#[reflect_trait]
pub trait ActionMeta {
	// fn system() -> SystemConfigs;
	fn graph_role(&self) -> GraphRole { GraphRole::Node }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	// use bevy::ecs::schedule::SystemConfigs;
	use bevy::reflect::ReflectFromPtr;
	use std::any::Any;
	use sweet::*;


	#[derive_action]
	#[action(graph_role = GraphRole::Node)]
	struct MyStruct;

	// impl ActionMeta for MyStruct {
	// 	fn system() -> SystemConfigs { my_struct.in_set(TickSet) }
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
