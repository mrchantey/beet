use crate::prelude::*;
use anyhow::Result;
use bevy::prelude::*;
use forky_core::ResultTEExt;
use serde::{de::DeserializeOwned, Serialize};


/// Functions for handling reception of [`Resource`] messages.
#[derive(Copy, Clone)]
pub struct ResourceFns {
	pub insert: fn(&mut Commands, payload: &MessagePayload) -> Result<()>,
	pub change: fn(&mut Commands, payload: &MessagePayload) -> Result<()>,
	pub remove: fn(&mut Commands),
}

impl ResourceFns {
	pub fn new<T: Resource + DeserializeOwned>() -> Self {
		Self {
			insert: |commands, payload| {
				commands.insert_resource(payload.deserialize::<T>()?);
				Ok(())
			},
			change: |commands, payload| {
				commands.insert_resource(payload.deserialize::<T>()?);
				Ok(())
			},
			remove: |commands| {
				commands.remove_resource::<T>();
			},
		}
	}
}

pub fn register_resource_outgoing<T: Resource + Serialize>(app: &mut App) {
	app.add_systems(Update, handle_outgoing::<T>.in_set(MessageOutgoingSet));
}

fn handle_outgoing<T: Resource + Serialize>(
	registrations: Res<ReplicateRegistry>,
	mut outgoing: ResMut<MessageOutgoing>,
	value: Option<Res<T>>,
	mut exists: Local<bool>,
) {
	if let Some(value) = value {
		if *exists && value.is_changed() {
			// CHANGED
			let Some(payload) =
				MessagePayload::new(&*value).ok_or(|e| log::error!("{e}"))
			else {
				return;
			};
			outgoing.push(
				Message::ChangeResource {
					reg_id: registrations.registration_id::<T>(),
					payload,
				}
				.into(),
			);
		} else {
			// ADDED
			*exists = true;
			let Some(payload) =
				MessagePayload::new(&*value).ok_or(|e| log::error!("{e}"))
			else {
				return;
			};
			outgoing.push(
				Message::InsertResource {
					reg_id: registrations.registration_id::<T>(),
					payload,
				}
				.into(),
			);
		}
	} else if *exists {
		// REMOVED
		*exists = false;
		outgoing.push(
			Message::RemoveResource {
				reg_id: registrations.registration_id::<T>(),
			}
			.into(),
		);
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


	#[derive(Debug, Clone, Resource, Serialize, Deserialize, PartialEq)]
	pub struct MyResource(pub i32);

	#[test]
	fn outgoing() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(ReplicatePlugin)
			.replicate_resource_outgoing::<MyResource>();

		app.world_mut().insert_resource(MyResource(7));
		app.update();

		app.world_mut().insert_resource(MyResource(8));
		app.update();

		app.world_mut().remove_resource::<MyResource>();
		app.update();

		let reg_id = RegistrationId::new_with(0);

		let msg_out = app.world_mut().resource_mut::<MessageOutgoing>();
		expect(msg_out.len()).to_be(3)?;
		expect(&msg_out[0]).to_be(
			&&Message::InsertResource {
				reg_id,
				payload: MessagePayload::new(&MyResource(7))?,
			}
			.into(),
		)?;
		expect(&msg_out[1]).to_be(
			&&Message::ChangeResource {
				reg_id: RegistrationId::new_with(0),
				payload: MessagePayload::new(&MyResource(8))?,
			}
			.into(),
		)?;
		expect(&msg_out[2])
			.to_be(&&Message::RemoveResource { reg_id }.into())?;

		Ok(())
	}

	#[test]
	fn incoming() -> Result<()> {
		let mut app1 = App::new();
		app1.add_plugins(ReplicatePlugin)
			.replicate_resource_outgoing::<MyResource>();

		let mut app2 = App::new();

		app2.add_plugins(ReplicatePlugin)
			.replicate_resource_incoming::<MyResource>();
		app1.world_mut().insert_resource(MyResource(7));
		app1.update();
		app1.world_mut().insert_resource(MyResource(8));

		app1.update();

		Message::loopback(app1.world_mut(), app2.world_mut());

		let msg_in = app2.world_mut().resource_mut::<MessageIncoming>();
		expect(msg_in.len()).to_be(2)?;

		expect(&msg_in[0]).to_be(
			&&Message::InsertResource {
				reg_id: RegistrationId::new_with(0),
				payload: MessagePayload::new(&MyResource(7))?,
			}
			.into(),
		)?;
		expect(&msg_in[1]).to_be(
			&&Message::ChangeResource {
				reg_id: RegistrationId::new_with(0),
				payload: MessagePayload::new(&MyResource(8))?,
			}
			.into(),
		)?;

		app2.update();

		expect(&app2).resource()?.to_be(&MyResource(8))?;

		Ok(())
	}
}
