use crate::prelude::*;
use anyhow::Result;
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use forky_core::ResultTEExt;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Functions for handling reception of [`Component`] messages.
#[derive(Copy, Clone)]
pub struct ComponentFns {
	// pub type_id: std::any::TypeId,
	pub insert: fn(&mut EntityCommands, payload: &MessagePayload) -> Result<()>,
	pub change: fn(&mut EntityCommands, payload: &MessagePayload) -> Result<()>,
	pub remove: fn(&mut EntityCommands),
}

impl ComponentFns {
	pub fn new<T: Component + DeserializeOwned>() -> Self {
		Self {
			insert: |commands, payload| {
				commands.insert(payload.deserialize::<T>()?);
				Ok(())
			},
			change: |commands, payload| {
				commands.insert(payload.deserialize::<T>()?);
				Ok(())
			},
			remove: |commands| {
				commands.remove::<T>();
			},
		}
	}
}

fn outgoing_add<T: Component + Serialize>(
	trigger: Trigger<OnAdd, T>,
	registrations: Res<ReplicateRegistry>,
	mut outgoing: ResMut<MessageOutgoing>,
	query: Query<&T, With<Replicate>>,
) {
	if let Ok(component) = query.get(trigger.entity()) {
		let Some(payload) =
			MessagePayload::new(component).ok_or(|e| log::error!("{e}"))
		else {
			return;
		};
		outgoing.push(
			Message::Add {
				entity: trigger.entity(),
				reg_id: registrations.registration_id::<T>(),
				payload,
			}
			.into(),
		);
	} else {
		// no replicate component
	}
}

/// This is a system because currently no `OnChange` trigger exists
fn outgoing_change<T: Component + Serialize>(
	registrations: Res<ReplicateRegistry>,
	mut outgoing: ResMut<MessageOutgoing>,
	query: Query<(Entity, Ref<T>), (Changed<T>, With<Replicate>)>,
) {
	for (entity, component) in query.iter() {
		if component.is_added() {
			continue;
		}
		let Some(payload) = MessagePayload::new(component.into_inner())
			.ok_or(|e| log::error!("{e}"))
		else {
			continue;
		};

		outgoing.push(
			Message::Change {
				entity,
				reg_id: registrations.registration_id::<T>(),
				payload,
			}
			.into(),
		);
	}
}

/// This only responds to removed componets, it ignores despawned entities
fn outgoing_remove<T: Component>(
	trigger: Trigger<OnRemove, T>,
	registrations: Res<ReplicateRegistry>,
	mut outgoing: ResMut<MessageOutgoing>,
	query: Query<(), With<Replicate>>,
) {
	if query.contains(trigger.entity()) {
		outgoing.push(
			Message::Remove {
				entity: trigger.entity(),
				reg_id: registrations.registration_id::<T>(),
			}
			.into(),
		);
	}
}

pub fn register_component_outgoing<T: Component + Serialize>(app: &mut App) {
	app.add_systems(Update, outgoing_change::<T>.in_set(MessageOutgoingSet));
	app.world_mut().observe(outgoing_add::<T>);
	app.world_mut().observe(outgoing_remove::<T>);
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
		expect(msg_out.len()).to_be(5)?;
		expect(&msg_out[0]).to_be(&Message::Spawn { entity }.into())?;
		expect(&msg_out[1]).to_be(
			&Message::Add {
				entity,
				reg_id: RegistrationId::new_with(0),
				payload: MessagePayload::new(&MyComponent(7))?,
			}
			.into(),
		)?;
		expect(&msg_out[2]).to_be(
			&Message::Change {
				entity,
				reg_id: RegistrationId::new_with(0),
				payload: MessagePayload::new(&MyComponent(8))?,
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
