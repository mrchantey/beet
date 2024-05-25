use crate::prelude::*;
use anyhow::Result;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use forky_core::ResultTEExt;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Copy, Clone)]
pub struct EventFns {
	pub send: fn(&mut World, payload: &MessagePayload) -> Result<()>,
}

impl<T: Send + Sync + 'static + Event + Serialize + DeserializeOwned>
	ReplicateType<ReplicateEventMarker> for T
{
	fn register(registrations: &mut ReplicateRegistry) {
		registrations.register_event::<T>(EventFns {
			send: |commands, payload| {
				commands.send_event(payload.deserialize::<T>()?);
				Ok(())
			},
		});
	}

	fn outgoing_systems() -> SystemConfigs { outgoing_send::<T>.into_configs() }
}
fn outgoing_send<T: Event + Serialize>(
	registrations: Res<ReplicateRegistry>,
	mut outgoing: ResMut<MessageOutgoing>,
	mut events: EventReader<T>,
) {
	for ev in events.read() {
		let Some(payload) =
			MessagePayload::new(ev).ok_or(|e| log::error!("{e}"))
		else {
			continue;
		};
		outgoing.push(
			Message::SendEvent {
				reg_id: registrations.registration_id::<T>(),
				payload,
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

		let msg_out = app.world_mut().resource_mut::<MessageOutgoing>();
		expect(msg_out.len()).to_be(1)?;
		expect(&msg_out[0]).to_be(
			&Message::SendEvent {
				reg_id: RegistrationId::new_with(0),
				payload: MessagePayload::new(&MyEvent(7))?,
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

		let msg_in = app2.world_mut().resource_mut::<MessageIncoming>();
		expect(msg_in.len()).to_be(1)?;

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
