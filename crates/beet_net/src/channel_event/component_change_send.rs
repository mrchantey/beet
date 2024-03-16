use crate::prelude::*;
use anyhow::Result;
use bevy::ecs::world::FilteredEntityRef;
use bevy::prelude::*;
use bevy::reflect::ReflectFromPtr;
use bevy::utils::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;


#[derive(Clone, Resource)]
pub struct ComponentChangeSend {
	pub type_registry: AppTypeRegistry,
	pub senders:
		Arc<RwLock<HashMap<EntityComponent, MpscChannel<ComponentChanged>>>>,
}

// #[derive(Component)]
// pub struct

impl ComponentChangeSend {
	pub fn new(type_registry: AppTypeRegistry) -> Self {
		Self {
			type_registry,
			senders: default(),
		}
	}

	pub fn register_recv<T: 'static>(&self, entity: Entity) {
		self.senders
			.write()
			.entry(EntityComponent::new::<T>(entity))
			.or_insert_with(default);
	}

	pub fn recv<T: Component + FromReflect>(
		&self,
		entity: Entity,
	) -> Result<Option<T>> {
		let type_registry = self.type_registry.read();
		let last_msg = self
			.senders
			.write()
			.get_mut(&EntityComponent::new::<T>(entity))
			.expect(
				"tried to get an unregistered channel, call `register_recv`",
			)
			.recv
			.try_recv_all()?
			.into_iter()
			.map(|t| t.deserialize_typed::<T>(&type_registry))
			.collect::<Result<Vec<T>>>()?
			.into_iter()
			.last();

		Ok(last_msg)
	}

	// https://bevyengine.org/news/bevy-0-13/#dynamic-queries
	pub fn system(world: &mut World) {
		let entity_components = world
			.resource::<ComponentChangeSend>()
			.senders
			.read()
			.iter()
			.map(|(k, v)| (k.clone(), v.send.clone()))
			.collect::<Vec<_>>();

		let this_run = world.change_tick();
		let last_run = world.last_change_tick();


		let failures = entity_components
			.into_iter()
			.filter_map(|(entity_component, sender)| {
				let EntityComponent { entity, type_id } = entity_component;

				let component_id = world.components().get_id(type_id).unwrap();

				if let Ok(entity_ref) =
					QueryBuilder::<FilteredEntityRef>::new(world)
						.ref_id(component_id)
						.build()
						.get(world, entity)
				{
					if let Some(ticks) =
						entity_ref.get_change_ticks_by_id(component_id)
					{
						if ticks.is_changed(last_run, this_run)
							|| ticks.is_added(last_run, this_run)
						{
							let type_registry =
								world.resource::<AppTypeRegistry>().read();
							let reflect_data =
								type_registry.get(type_id).unwrap();
							let reflect_from_ptr = reflect_data
								.data::<ReflectFromPtr>()
								.unwrap()
								.clone();

							let value =
								entity_ref.get_by_id(component_id).unwrap();
							let value =
								unsafe { reflect_from_ptr.as_reflect(value) };


							let msg = ComponentChanged::serialize(
								&type_registry,
								entity,
								value,
								type_id,
							)
							.unwrap();

							return sender
								.send(msg)
								.map_err(|_| entity_component)
								.err();
						}
					}
				}
				None
			})
			.collect::<Vec<_>>();

		for failure in failures.into_iter().rev() {
			world
				.resource::<ComponentChangeSend>()
				.senders
				.write()
				.remove(&failure);
		}
	}
}


#[cfg(test)]
mod test {
	use super::*;
	use anyhow::Result;
	use sweet::*;

	#[derive(Debug, Clone, PartialEq, Component, Reflect)]
	struct MyStruct(pub i32);

	#[test]
	fn test_component_changed_send() -> Result<()> {
		let mut app = App::new();

		app.add_systems(PreUpdate, ComponentChangeSend::system);
		let registry = app.world.resource::<AppTypeRegistry>().clone();
		registry.write().register::<MyStruct>();
		app.world.init_component::<MyStruct>();

		let send = ComponentChangeSend::new(registry);
		app.insert_resource(send.clone());

		let entity = app.world.spawn(MyStruct(2)).id();
		send.register_recv::<MyStruct>(entity);
		app.update();
		expect(send.recv(entity)?).as_some()?.to_be(MyStruct(2))?;
		app.update();
		expect(send.recv::<MyStruct>(entity)?).to_be_none()?;
		app.world.entity_mut(entity).insert(MyStruct(3));
		app.update();
		expect(send.recv(entity)?).as_some()?.to_be(MyStruct(3))?;

		Ok(())
	}
}
