use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use forky_core::ResultTEExt;
use serde::de::DeserializeOwned;
use serde::Serialize;


#[derive(Copy, Clone)]
pub struct ResourceFns {
	pub insert: fn(&mut Commands, payload: &[u8]) -> bincode::Result<()>,
	pub remove: fn(&mut Commands) -> bincode::Result<()>,
}

impl<T: Send + Sync + 'static + Resource + Serialize + DeserializeOwned>
	ReplicateType<ReplicateResourceMarker> for T
{
	fn register(registrations: &mut Registrations) {
		registrations.register_resource::<T>(ResourceFns {
			insert: |commands, payload| {
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
	registrations: Res<Registrations>,
	mut outgoing: ResMut<MessageOutgoing>,
	value: Option<Res<T>>,
	mut exists: Local<bool>,
) {
	if let Some(value) = value {
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
	} else if *exists {
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
		app.world_mut().remove_resource::<MyResource>();
		app.update();

		let reg_id = RegistrationId::new_with(0);

		let events = app.world_mut().resource_mut::<MessageOutgoing>();
		expect(events.len()).to_be(2)?;
		expect(&events[0]).to_be(
			&&Message::InsertResource {
				reg_id,
				bytes: vec![7, 0, 0, 0],
			}
			.into(),
		)?;
		expect(&events[1])
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

		Message::loopback(app1.world_mut(), app2.world_mut());

		app2.update();

		expect(&app2).resource()?.to_be(&MyResource(7))?;

		Ok(())
	}
}
