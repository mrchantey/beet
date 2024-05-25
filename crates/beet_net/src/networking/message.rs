use crate::prelude::RegistrationId;
use bevy::prelude::*;
use serde::de::DeserializeOwned;
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
		entity: Entity,
		payload: MessagePayload,
	},
	Change {
		reg_id: RegistrationId,
		entity: Entity,
		payload: MessagePayload,
	},
	Remove {
		reg_id: RegistrationId,
		entity: Entity,
	},
	InsertResource {
		reg_id: RegistrationId,
		payload: MessagePayload,
	},
	ChangeResource {
		reg_id: RegistrationId,
		payload: MessagePayload,
	},
	RemoveResource {
		reg_id: RegistrationId,
	},
	SendEvent {
		reg_id: RegistrationId,
		payload: MessagePayload,
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

	pub fn from_bytes(bytes: &[u8]) -> bincode::Result<Vec<Message>> {
		bincode::deserialize::<Vec<Message>>(bytes)
	}

	pub fn into_bytes(items: &Vec<Message>) -> bincode::Result<Vec<u8>> {
		bincode::serialize(items)
	}

	#[cfg(feature = "serde_json")]
	pub fn from_json(json: &str) -> serde_json::Result<Vec<Message>> {
		serde_json::from_str::<Vec<Message>>(json)
	}

	#[cfg(feature = "serde_json")]
	pub fn into_json(items: &Vec<Message>) -> serde_json::Result<String> {
		serde_json::to_string(items)
	}
}

#[derive(Debug, Clone, PartialEq, Resource, Serialize, Deserialize)]
pub enum MessagePayload {
	Bytes(Vec<u8>),
	Json(String),
}

impl MessagePayload {
	pub fn bytes<T: Serialize>(value: T) -> bincode::Result<Self> {
		let bytes = bincode::serialize(&value)?;
		Ok(Self::Bytes(bytes))
	}
	#[cfg(feature = "serde_json")]
	pub fn json<T: Serialize>(value: T) -> serde_json::Result<Self> {
		let json = serde_json::to_string(&value)?;
		Ok(Self::Json(json))
	}
	pub fn deserialize<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
		match self {
			Self::Bytes(bytes) => Ok(bincode::deserialize(bytes)?),
			Self::Json(json) => {
				#[cfg(feature = "serde_json")]
				return Ok(serde_json::from_str(json)?);
				#[cfg(not(feature = "serde_json"))]
				anyhow::bail!("message payload is json but `serde_json` feature is not enabled")
			}
		}
	}
}
