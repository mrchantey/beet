use super::*;
use crate::utils::flumeReceiverTExt;
use anyhow::Result;
use bevy::ecs::world::FilteredEntityMut;
use bevy::prelude::*;
use bevy::reflect::ReflectFromPtr;
use bevy::utils::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone, Resource)]
pub struct ComponentChangeRecv {
	pub type_registry: AppTypeRegistry,
	pub receivers:
		Arc<RwLock<HashMap<EntityComponent, MpscChannel<ComponentChanged>>>>,
}


impl ComponentChangeRecv {
	pub fn new(type_registry: AppTypeRegistry) -> Self {
		Self {
			type_registry,
			receivers: default(),
		}
	}

	pub fn send<T: Component + Reflect>(
		&self,
		entity: Entity,
		value: &T,
	) -> Result<()> {
		let msg = ComponentChanged::serialize_typed(
			&self.type_registry.read(),
			entity,
			value,
		)?;
		self.receivers
			.write()
			.entry(msg.id)
			.or_insert_with(default)
			.send
			.send(msg)?;
		Ok(())
	}

	pub fn system(world: &mut World) {
		let (messages, fails): (
			Vec<Option<Vec<ComponentChanged>>>,
			Vec<Option<EntityComponent>>,
		) = world
			.resource::<ComponentChangeRecv>()
			.receivers
			.write()
			.iter_mut()
			.map(|(entity, channel)| match channel.recv.try_recv_all() {
				Ok(msg) => (Some(msg), None),
				Err(_) => (None, Some(*entity)),
			})
			.unzip();

		for changed in messages.into_iter().flatten().flatten() {
			let type_id = changed.id.type_id;
			let entity = changed.id.entity;
			let component_id = world.components().get_id(type_id).unwrap();
			let type_registry = world.resource::<AppTypeRegistry>().read();
			let reflect_data = type_registry.get(type_id).unwrap();
			let reflect_from_ptr =
				reflect_data.data::<ReflectFromPtr>().unwrap().clone();

			let new_value = changed.deserialize(&type_registry).unwrap();

			drop(type_registry);

			if let Ok(mut entity_mut) =
				QueryBuilder::<FilteredEntityMut>::new(world)
					.mut_id(component_id)
					.build()
					.get_mut(world, entity)
			{
				let mut value = entity_mut.get_mut_by_id(component_id).unwrap();
				let value =
					unsafe { reflect_from_ptr.as_reflect_mut(value.as_mut()) };


				value.apply(new_value.as_ref());
			}
		}

		for fail in fails.into_iter().flatten().rev() {
			world
				.resource::<ComponentChangeRecv>()
				.receivers
				.write()
				.remove(&fail);
		}
		// Ok(())
	}
}



#[cfg(test)]

mod test {
	use super::*;
	use anyhow::Result;
	use sweet::*;

	#[derive(Debug, PartialEq, Component, Reflect)]
	struct MyStruct(pub i32);

	#[test]
	fn test_component_changed_recv() -> Result<()> {
		let mut app = App::new();

		app.add_systems(PreUpdate, ComponentChangeRecv::system);
		let registry = app.world.resource::<AppTypeRegistry>().clone();
		registry.write().register::<MyStruct>();
		app.world.init_component::<MyStruct>();

		let recv = ComponentChangeRecv::new(registry);
		app.insert_resource(recv.clone());

		let entity = app.world.spawn(MyStruct(0)).id();
		recv.send(entity, &MyStruct(2))?;
		app.update();
		expect(&app).component(entity)?.to_be(&MyStruct(2))?;

		Ok(())
	}
}
