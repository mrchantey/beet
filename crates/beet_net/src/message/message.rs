use crate::prelude::RegistrationId;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;


#[derive(Debug, Clone, PartialEq, Deref, DerefMut, Event)]
pub struct MessageIncoming(pub Message);

impl Into<MessageIncoming> for Message {
	fn into(self) -> MessageIncoming { MessageIncoming(self) }
}
impl Into<MessageIncoming> for MessageOutgoing {
	fn into(self) -> MessageIncoming { MessageIncoming(self.0) }
}

#[derive(Debug, Clone, PartialEq, Deref, DerefMut, Event)]
pub struct MessageOutgoing(pub Message);

impl Into<MessageOutgoing> for Message {
	fn into(self) -> MessageOutgoing { MessageOutgoing(self) }
}

impl Into<MessageOutgoing> for MessageIncoming {
	fn into(self) -> MessageOutgoing { MessageOutgoing(self.0) }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Message {
	Spawn {
		entity: Entity,
	},
	Despawn {
		entity: Entity,
	},
	Insert {
		entity: Entity,
		reg_id: RegistrationId,
		bytes: Vec<u8>,
	},
	Change {
		entity: Entity,
		reg_id: RegistrationId,
		bytes: Vec<u8>,
	},
	Remove {
		entity: Entity,
		reg_id: RegistrationId,
	},
	// InsertResource {
	// 	resource_id: SerdeComponentId,
	// 	bytes: Vec<u8>,
	// },
}

impl Message {
	pub fn loopback(outgoing: &mut World, incoming: &mut World) {
		for event in outgoing
			.resource::<Events<MessageOutgoing>>()
			.iter_current_update_events()
		{
			incoming.send_event(MessageIncoming(event.0.clone()));
		}
	}
}


#[extend::ext]
pub impl World {
	#[cfg(test)]
	fn events<T: Event>(&self) -> Vec<&T> {
		self.resource::<Events<T>>()
			.iter_current_update_events()
			.collect()
	}
}
