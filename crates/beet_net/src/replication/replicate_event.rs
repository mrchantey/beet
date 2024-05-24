use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use forky_core::ResultTEExt;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Copy, Clone)]
pub struct EventFns {
	pub send: fn(&mut World, payload: &[u8]) -> bincode::Result<()>,
}

impl<T: Send + Sync + 'static + Event + Serialize + DeserializeOwned>
	ReplicateType<ReplicateEventMarker> for T
{
	fn register(registrations: &mut Registrations) {
		registrations.register_event::<T>(EventFns {
			send: |commands, payload| {
				let ev: T = bincode::deserialize(payload)?;
				commands.send_event(ev);
				Ok(())
			},
		});
	}

	fn outgoing_systems() -> SystemConfigs { outgoing_send::<T>.into_configs() }
}
fn outgoing_send<T: Event + Serialize>(
	registrations: Res<Registrations>,
	mut outgoing: ResMut<MessageOutgoing>,
	mut events: EventReader<T>,
) {
	for ev in events.read() {
		let Some(bytes) = bincode::serialize(ev).ok_or(|e| log::error!("{e}"))
		else {
			continue;
		};
		outgoing.push(
			Message::SendEvent {
				reg_id: registrations.registration_id::<T>(),
				bytes,
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

	#[derive(Debug, Clone, Event, Serialize, Deserialize, PartialEq)]
	pub struct MyEvent(pub i32);

	#[test]
	fn outgoing() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(ReplicatePlugin)
			.add_event::<MyEvent>()
			.replicate_event::<MyEvent>();

		app.world_mut().send_event(MyEvent(7));

		app.update();

		let events = app.world_mut().resource_mut::<MessageOutgoing>();
		expect(events.len()).to_be(1)?;
		expect(&events[0]).to_be(
			&Message::SendEvent {
				reg_id: RegistrationId::new_with(0),
				bytes: vec![7, 0, 0, 0],
			}
			.into(),
		)?;

		Ok(())
	}

	#[test]
	fn incoming() -> Result<()> {
		let mut app1 = App::new();
		app1.add_plugins(ReplicatePlugin)
			.add_event::<MyEvent>()
			.replicate_event::<MyEvent>();
		let mut app2 = App::new();

		app2.add_plugins(ReplicatePlugin)
			.add_event::<MyEvent>()
			.replicate_event::<MyEvent>();


		// Send
		app1.world_mut().send_event(MyEvent(7));
		app1.update();
		Message::loopback(app1.world_mut(), app2.world_mut());

		let events = app2.world_mut().resource_mut::<MessageIncoming>();
		expect(events.len()).to_be(1)?;

		app2.update();
		let events = app2
			.world_mut()
			.resource::<Events<MyEvent>>()
			.iter_current_update_events()
			.collect::<Vec<_>>();

		expect(events.len()).to_be(1)?;
		expect(events[0]).to_be(&MyEvent(7))?;
		Ok(())
	}
}
