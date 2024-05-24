use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use forky_core::ResultTEExt;
use serde::de::DeserializeOwned;
use serde::Serialize;


#[derive(Copy, Clone)]
pub struct ResourceFns {
	pub insert: fn(&mut Commands, payload: &[u8]) -> bincode::Result<()>,
	pub change: fn(&mut Commands, payload: &[u8]) -> bincode::Result<()>,
	pub remove: fn(&mut Commands) -> bincode::Result<()>,
}

impl<T: Send + Sync + 'static + Resource + Serialize + DeserializeOwned>
	ReplicateType<ReplicateResourceMarker> for T
{
	fn register(registrations: &mut ReplicateRegistry) {
		registrations.register_resource::<T>(ResourceFns {
			insert: |commands, payload| {
				let res: T = bincode::deserialize(payload)?;
				commands.insert_resource(res);
				Ok(())
			},
			change: |commands, payload| {
				let res: T = bincode::deserialize(payload)?;
				commands.insert_resource(res);
				Ok(())
			},
			remove: |commands| {
				commands.remove_resource::<T>();
				Ok(())
			},
		});
	}

	fn outgoing_systems() -> SystemConfigs {
		handle_outgoing::<T>.into_configs()
	}
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
			let Some(bytes) =
				bincode::serialize(&*value).ok_or(|e| log::error!("{e}"))
			else {
				return;
			};
			outgoing.push(
				Message::ChangeResource {
					reg_id: registrations.registration_id::<T>(),
					bytes,
				}
				.into(),
			);
		} else {
			// ADDED
			*exists = true;
			let Some(bytes) =
				bincode::serialize(&*value).ok_or(|e| log::error!("{e}"))
			else {
				return;
			};
			outgoing.push(
				Message::InsertResource {
					reg_id: registrations.registration_id::<T>(),
					bytes,
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
			.replicate_resource::<MyResource>();

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
				bytes: vec![7, 0, 0, 0],
			}
			.into(),
		)?;
		expect(&msg_out[1]).to_be(
			&&Message::ChangeResource {
				reg_id: RegistrationId::new_with(0),
				bytes: vec![8, 0, 0, 0],
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
			.replicate_resource::<MyResource>();

		let mut app2 = App::new();

		app2.add_plugins(ReplicatePlugin)
			.replicate_resource::<MyResource>();
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
				bytes: vec![7, 0, 0, 0],
			}
			.into(),
		)?;
		expect(&msg_in[1]).to_be(
			&&Message::ChangeResource {
				reg_id: RegistrationId::new_with(0),
				bytes: vec![8, 0, 0, 0],
			}
			.into(),
		)?;

		app2.update();

		expect(&app2).resource()?.to_be(&MyResource(8))?;

		Ok(())
	}
}