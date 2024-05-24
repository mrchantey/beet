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

// impl<T: Send + Sync + 'static + Event + Serialize + DeserializeOwned>
// 	ReplicateType for T
// {
// 	fn register(registrations: &mut Registrations) {
// 		registrations.register_event::<T>(EventFns {
// 			send: |commands, payload| {
// 				let ev: T = bincode::deserialize(payload)?;
// 				commands.send_event(ev);
// 				Ok(())
// 			},
// 		});
// 	}

// 	fn outgoing_systems() -> SystemConfigs { outgoing_send::<T>.into_configs() }
// }
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
