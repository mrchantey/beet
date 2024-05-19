use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use forky_core::ResultTEExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;


pub struct ReplicateComponentPlugin<T: ReplicateComponent> {
	phantom: PhantomData<T>,
}

impl<T: ReplicateComponent> Default for ReplicateComponentPlugin<T> {
	fn default() -> Self {
		Self {
			phantom: PhantomData,
		}
	}
}


impl<T: ReplicateComponent> Plugin for ReplicateComponentPlugin<T> {
	fn build(&self, app: &mut App) {
		app.init_resource::<Registrations>();
		let mut registrations = app.world_mut().resource_mut::<Registrations>();
		T::register(&mut registrations);
		app.add_systems(Update, T::update_systems().after(MessageSet));
	}
}


pub struct ComponentFns {
	pub type_id: std::any::TypeId,
	pub insert: fn(&mut EntityCommands, payload: &[u8]) -> bincode::Result<()>,
	pub change: fn(&mut EntityCommands, payload: &[u8]) -> bincode::Result<()>,
	pub remove: fn(&mut EntityCommands),
}

fn outgoing_insert<T: Component + Serialize>(
	registrations: Res<Registrations>,
	mut outgoing: EventWriter<MessageOutgoing>,
	query: Query<(Entity, &T), (Added<T>, With<Replicate>)>,
) {
	for (entity, component) in query.iter() {
		let Some(bytes) =
			bincode::serialize(component).ok_or(|e| log::error!("{e}"))
		else {
			continue;
		};
		outgoing.send(
			Message::Insert {
				entity,
				reg_id: registrations.registration_id::<T>(),
				bytes,
			}
			.into(),
		);
	}
}

fn outgoing_change<T: Component + Serialize>(
	registrations: Res<Registrations>,
	mut outgoing: EventWriter<MessageOutgoing>,
	query: Query<(Entity, Ref<T>), (Changed<T>, With<Replicate>)>,
) {
	for (entity, component) in query.iter() {
		if component.is_added() {
			continue;
		}
		let Some(bytes) = bincode::serialize(component.into_inner())
			.ok_or(|e| log::error!("{e}"))
		else {
			continue;
		};

		outgoing.send(
			Message::Change {
				entity,
				reg_id: registrations.registration_id::<T>(),
				bytes,
			}
			.into(),
		);
	}
}

/// Ignores despawned entities
fn handle_remove<T: Component>(
	registrations: Res<Registrations>,
	mut outgoing: EventWriter<MessageOutgoing>,
	mut removed: RemovedComponents<T>,
	query: Query<(), With<Replicate>>,
) {
	for removed in removed.read() {
		if query.contains(removed) {
			outgoing.send(
				Message::Remove {
					entity: removed,
					reg_id: registrations.registration_id::<T>(),
				}
				.into(),
			);
		}
	}
}


pub trait ReplicateComponent: 'static + Send + Sync {
	fn register(registrations: &mut Registrations);
	fn update_systems() -> SystemConfigs;
}

impl<T: Send + Sync + 'static + Component + Serialize + DeserializeOwned>
	ReplicateComponent for T
{
	fn register(registrations: &mut Registrations) {
		registrations.register_component(ComponentFns {
			type_id: std::any::TypeId::of::<T>(),
			insert: |commands, payload| {
				let component: T = bincode::deserialize(payload)?;
				commands.insert(component);
				Ok(())
			},
			change: |commands, payload| {
				let component: T = bincode::deserialize(payload)?;
				commands.insert(component);
				Ok(())
			},
			remove: |commands| {
				commands.remove::<T>();
			},
		});
	}

	fn update_systems() -> SystemConfigs {
		(
			outgoing_insert::<T>,
			outgoing_change::<T>,
			handle_remove::<T>,
		)
			.into_configs()
	}
}

// macro_rules! impl_replicate_component_tuples {
// 	($($param: ident),*) => {
// 			impl<$($param),*> ReplicateComponent for ($($param,)*)
// 			where
// 					$($param: Send + Sync + 'static + Component + Serialize + DeserializeOwned),*
// 			{
// 					#[allow(non_snake_case, unused_variables)]
// 					#[track_caller]
// 					fn register(registrations:&mut Registrations) {
// 							$($param::register(registrations);)*
// 					}
// 					fn update_systems()-> SystemConfigs {
// 							($($param::update_systems(),)*).into_configs()
// 					}
// 			}
// 	}
// }
// all_tuples!(impl_replicate_component_tuples, 1, 15, P);


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use serde::Deserialize;
	use serde::Serialize;
	use sweet::*;

	#[derive(Debug, Clone, Component, Serialize, Deserialize, PartialEq)]
	pub struct MyComponent(pub i32);

	#[test]
	fn outgoing() -> Result<()> {
		let mut app = App::new();
		app.add_plugins((
			ReplicatePlugin,
			ReplicateComponentPlugin::<MyComponent>::default(),
		));

		let entity = app
			.world_mut()
			.spawn((Replicate::default(), MyComponent(7)))
			.id();

		app.update();
		let events = app.world_mut().events::<MessageOutgoing>();
		expect(events.len()).to_be(2)?;
		expect(events[0]).to_be(&Message::Spawn { entity }.into())?;
		expect(events[1]).to_be(
			&Message::Insert {
				entity,
				reg_id: RegistrationId::new_with(0),
				bytes: vec![7, 0, 0, 0],
			}
			.into(),
		)?;


		app.world_mut().entity_mut(entity).insert(MyComponent(8));

		app.update();

		let events = app.world_mut().events::<MessageOutgoing>();
		expect(events.len()).to_be(1)?;
		expect(events[0]).to_be(
			&Message::Change {
				entity,
				reg_id: RegistrationId::new_with(0),
				bytes: vec![8, 0, 0, 0],
			}
			.into(),
		)?;

		app.world_mut().despawn(entity);

		app.update();
		let events = app.world_mut().events::<MessageOutgoing>();
		expect(events.len()).to_be(1)?;
		expect(events[0]).to_be(&Message::Despawn { entity }.into())?;

		Ok(())
	}

	#[test]
	fn incoming() -> Result<()> {
		let mut app1 = App::new();
		app1.add_plugins((
			ReplicatePlugin,
			ReplicateComponentPlugin::<MyComponent>::default(),
		));

		let entity1 = app1
			.world_mut()
			.spawn((Replicate::default(), MyComponent(7)))
			.id();

		app1.update();

		let mut app2 = App::new();

		app2.add_plugins((
			ReplicatePlugin,
			ReplicateComponentPlugin::<MyComponent>::default(),
		));

		Message::loopback(app1.world_mut(), app2.world_mut());

		app2.update();

		expect(
			app2.world_mut()
				.query::<&MyComponent>()
				.iter(app2.world())
				.next(),
		)
		.as_some()?
		.to_be(&MyComponent(7))?;

		app1.world_mut()
			.entity_mut(entity1)
			.get_mut::<MyComponent>()
			.unwrap()
			.0 = 8;

		app1.update();


		Message::loopback(app1.world_mut(), app2.world_mut());

		// let events = app2
		// 	.world_mut()
		// 	.run_system_once(collect_events::<MessageOutgoing>);
		// expect(events.len()).to_be(3)?;
		// expect(&events[2]).to_be(
		// 	&Message::Change {
		// 		entity: entity1,
		// 		reg_id: RegistrationId::new_with(0),
		// 		bytes: vec![8, 0, 0, 0],
		// 	}
		// 	.into(),
		// )?;

		app2.update();

		expect(
			app2.world_mut()
				.query::<&MyComponent>()
				.iter(app2.world())
				.next(),
		)
		.as_some()?
		.to_be(&MyComponent(8))?;


		Ok(())
	}
}
