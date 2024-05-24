use crate::prelude::RegistrationId;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;


#[derive(Debug, Default, Clone, PartialEq, Deref, DerefMut, Resource)]
pub struct MessageIncoming(pub Vec<Message>);


#[derive(Debug, Default, Clone, PartialEq, Deref, DerefMut, Resource)]
pub struct MessageOutgoing(pub Vec<Message>);


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Message {
	Spawn {
		entity: Entity,
	},
	Despawn {
		entity: Entity,
	},
	Insert {
		reg_id: RegistrationId,
		bytes: Vec<u8>,
		entity: Entity,
	},
	Change {
		reg_id: RegistrationId,
		entity: Entity,
		bytes: Vec<u8>,
	},
	Remove {
		reg_id: RegistrationId,
		entity: Entity,
	},
	InsertResource {
		reg_id: RegistrationId,
		bytes: Vec<u8>,
	},
	ChangeResource {
		reg_id: RegistrationId,
		bytes: Vec<u8>,
	},
	RemoveResource {
		reg_id: RegistrationId,
	},
	SendEvent {
		reg_id: RegistrationId,
		bytes: Vec<u8>,
	},
}

impl Message {
	/// Clear outgoing and drain incoming into outgoing messages.
	pub fn loopback(outgoing: &mut World, incoming: &mut World) {
		incoming.resource_mut::<MessageIncoming>().0 = outgoing
			.resource_mut::<MessageOutgoing>()
			.drain(..)
			.collect();
	}

	pub fn from_bytes(bytes: &[u8]) -> Result<Vec<Message>, bincode::Error> {
		bincode::deserialize::<Vec<Message>>(bytes)
	}

	pub fn into_bytes(items: &Vec<Message>) -> Result<Vec<u8>, bincode::Error> {
		bincode::serialize(items)
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
