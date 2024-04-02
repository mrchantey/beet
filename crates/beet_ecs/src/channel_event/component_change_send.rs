use crate::prelude::*;
use bevy::ecs::world::FilteredEntityRef;
use bevy::prelude::*;
use bevy::reflect::ReflectFromPtr;
use bevy::utils::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;


#[derive(Clone, Resource)]
pub struct ComponentChangeSend {
	pub type_registry: AppTypeRegistry,
	pub senders: Arc<
		RwLock<
			HashMap<
				EntityComponent,
				Vec<Box<dyn 'static + Send + Sync + Fn(ComponentChanged)>>,
			>,
		>,
	>,
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

	pub fn add<T: 'static + FromReflect>(
		&self,
		entity: Entity,
		handler: impl 'static + Send + Sync + Fn(T),
	) {
		let type_registry = self.type_registry.clone();
		let func = Box::new(move |msg: ComponentChanged| {
			let msg =
				msg.deserialize_typed::<T>(&type_registry.read()).unwrap();
			handler(msg);
		});

		self.senders
			.write()
			.entry(EntityComponent::new::<T>(entity))
			.or_insert_with(default)
			.push(func);
	}

	// https://bevyengine.org/news/bevy-0-13/#dynamic-queries
	pub fn system(world: &mut World) {
		let this_run = world.read_change_tick();
		let last_run = world.last_change_tick();


		let entity_components = world
			.resource::<ComponentChangeSend>()
			.senders
			.read()
			.keys()
			.cloned()
			.collect::<Vec<_>>();
		let messages = entity_components
			.into_iter()
			.filter_map(|EntityComponent { entity, type_id }| {
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
							return Some(msg);
						}
					}
				}
				None
			})
			.collect::<Vec<_>>();

		let sender_map = world.resource::<ComponentChangeSend>().senders.read();
		for message in messages {
			let senders = sender_map.get(&message.id).unwrap();
			for sender in senders {
				sender(message.clone());
			}
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

		app.add_systems(PostUpdate, ComponentChangeSend::system);
		let registry = app.world().resource::<AppTypeRegistry>().clone();
		registry.write().register::<MyStruct>();
		app.world_mut().init_component::<MyStruct>();

		let send = ComponentChangeSend::new(registry);
		app.insert_resource(send.clone());

		let entity = app.world_mut().spawn(MyStruct(2)).id();

		let val = mock_value();

		let val2 = val.clone();
		send.add(entity, move |msg: MyStruct| {
			val2.push(msg);
		});

		app.update();
		expect(&val).to_contain(MyStruct(2))?;
		app.update();
		expect(&val).to_be_empty()?;
		app.world_mut().entity_mut(entity).insert(MyStruct(3));
		app.update();
		expect(&val).to_contain(MyStruct(3))?;

		Ok(())
	}
}
