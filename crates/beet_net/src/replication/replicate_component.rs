use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use forky_core::ResultTEExt;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Copy, Clone)]
pub struct ComponentFns {
	// pub type_id: std::any::TypeId,
	pub insert: fn(&mut EntityCommands, payload: &[u8]) -> bincode::Result<()>,
	pub change: fn(&mut EntityCommands, payload: &[u8]) -> bincode::Result<()>,
	pub remove: fn(&mut EntityCommands),
}

fn outgoing_insert<T: Component + Serialize>(
	registrations: Res<Registrations>,
	mut outgoing: ResMut<MessageOutgoing>,
	query: Query<(Entity, &T), (Added<T>, With<Replicate>)>,
) {
	for (entity, component) in query.iter() {
		let Some(bytes) =
			bincode::serialize(component).ok_or(|e| log::error!("{e}"))
		else {
			continue;
		};
		outgoing.push(
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
	mut outgoing: ResMut<MessageOutgoing>,
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

		outgoing.push(
			Message::Change {
				entity,
				reg_id: registrations.registration_id::<T>(),
				bytes,
			}
			.into(),
		);
	}
}

/// This only responds to removed componets, it ignores despawned entities
fn outgoing_remove<T: Component>(
	registrations: Res<Registrations>,
	mut outgoing: ResMut<MessageOutgoing>,
	mut removed: RemovedComponents<T>,
	query: Query<(), With<Replicate>>,
) {
	for removed in removed.read() {
		if query.contains(removed) {
			outgoing.push(
				Message::Remove {
					entity: removed,
					reg_id: registrations.registration_id::<T>(),
				}
				.into(),
			);
		}
	}
}

impl<T: Send + Sync + 'static + Component + Serialize + DeserializeOwned>
	ReplicateType<ReplicateComponentMarker> for T
{
	fn register(registrations: &mut Registrations) {
		registrations.register_component::<T>(ComponentFns {
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

	fn outgoing_systems() -> SystemConfigs {
		(
			outgoing_insert::<T>,
			outgoing_change::<T>,
			outgoing_remove::<T>,
		)
			.into_configs()
	}
}

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
		app.add_plugins(ReplicatePlugin).replicate::<MyComponent>();

		let entity = app
			.world_mut()
			.spawn((Replicate::default(), MyComponent(7)))
			.id();

		app.update();


		app.world_mut().entity_mut(entity).insert(MyComponent(8));

		app.update();


		app.world_mut().despawn(entity);

		app.update();

		let msg_out = app.world_mut().resource_mut::<MessageOutgoing>();
		expect(msg_out.len()).to_be(4)?;
		expect(&msg_out[0]).to_be(&Message::Spawn { entity }.into())?;
		expect(&msg_out[1]).to_be(
			&Message::Insert {
				entity,
				reg_id: RegistrationId::new_with(0),
				bytes: vec![7, 0, 0, 0],
			}
			.into(),
		)?;
		expect(&msg_out[2]).to_be(
			&Message::Change {
				entity,
				reg_id: RegistrationId::new_with(0),
				bytes: vec![8, 0, 0, 0],
			}
			.into(),
		)?;
		expect(&msg_out[3]).to_be(&Message::Despawn { entity }.into())?;

		Ok(())
	}

	#[test]
	fn incoming() -> Result<()> {
		let mut app1 = App::new();
		app1.add_plugins(ReplicatePlugin).replicate::<MyComponent>();
		let mut app2 = App::new();

		app2.add_plugins(ReplicatePlugin).replicate::<MyComponent>();


		// INSERT
		let entity1 = app1
			.world_mut()
			.spawn((Replicate::default(), MyComponent(7)))
			.id();
		app1.update();
		Message::loopback(app1.world_mut(), app2.world_mut());

		let msg_in = app2.world_mut().resource_mut::<MessageIncoming>();
		expect(msg_in.len()).to_be(2)?;

		app2.update();
		expect(
			app2.world_mut()
				.query::<&MyComponent>()
				.iter(app2.world())
				.next(),
		)
		.as_some()?
		.to_be(&MyComponent(7))?;

		// CHANGE
		app1.world_mut().entity_mut(entity1).insert(MyComponent(8));
		app1.update();
		Message::loopback(app1.world_mut(), app2.world_mut());
		app2.update();
		expect(
			app2.world_mut()
				.query::<&MyComponent>()
				.iter(app2.world())
				.next(),
		)
		.as_some()?
		.to_be(&MyComponent(8))?;

		// REMOVE
		app1.world_mut().entity_mut(entity1).remove::<MyComponent>();
		app1.update();
		Message::loopback(app1.world_mut(), app2.world_mut());
		app2.update();
		expect(
			app2.world_mut()
				.query::<&MyComponent>()
				.iter(app2.world())
				.next(),
		)
		.to_be_none()?;

		Ok(())
	}
}
