use crate::prelude::RegistrationId;
use anyhow::Result;
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
	Add {
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
	SendObserver {
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

	pub fn vec_from_bytes(bytes: &[u8]) -> bincode::Result<Vec<Message>> {
		bincode::deserialize::<Vec<Message>>(bytes)
	}

	pub fn vec_into_bytes(items: &Vec<Message>) -> Result<Vec<u8>> {
		let items = items
			.iter()
			.map(|m| m.with_bytes_payload())
			.collect::<Result<Vec<_>>>()?;

		let bytes = bincode::serialize(&items)?;
		Ok(bytes)
	}

	#[cfg(feature = "serde_json")]
	pub fn vec_from_json(json: &str) -> serde_json::Result<Vec<Message>> {
		serde_json::from_str::<Vec<Message>>(json)
	}

	#[cfg(feature = "serde_json")]
	pub fn vec_into_json(items: &Vec<Message>) -> Result<String> {
		let items = items
			.iter()
			.map(|m| m.with_json_payload())
			.collect::<Result<Vec<_>>>()?;

		let json = serde_json::to_string(&items)?;
		Ok(json)
	}

	fn with_payload(
		&self,
		func: impl FnOnce(&MessagePayload) -> Result<MessagePayload>,
	) -> Result<Self> {
		match self {
			Self::Add {
				entity,
				reg_id,
				payload,
			} => Ok(Self::Add {
				entity: *entity,
				reg_id: *reg_id,
				payload: func(payload)?,
			}),
			Self::Change {
				entity,
				reg_id,
				payload,
			} => Ok(Self::Change {
				entity: *entity,
				reg_id: *reg_id,
				payload: func(payload)?,
			}),
			Self::InsertResource { reg_id, payload } => {
				Ok(Self::InsertResource {
					reg_id: *reg_id,
					payload: func(payload)?,
				})
			}
			Self::ChangeResource { reg_id, payload } => {
				Ok(Self::ChangeResource {
					reg_id: *reg_id,
					payload: func(payload)?,
				})
			}
			Self::SendEvent { reg_id, payload } => Ok(Self::SendEvent {
				reg_id: *reg_id,
				payload: func(payload)?,
			}),
			other => Ok(other.clone()),
		}
	}

	pub fn with_bytes_payload(&self) -> Result<Self> {
		self.with_payload(|payload| payload.into_bytes())
	}
	pub fn with_json_payload(&self) -> Result<Self> {
		self.with_payload(|payload| payload.into_json())
	}
}



/// A serializable container for message payloads.
/// With the `serde_json` feature enabled, both binary and json representations are stored
/// and filtered depending on whether [`Message::vec_into_json`] or [`Message::vec_into_bytes`] is called.
#[derive(Debug, Clone, PartialEq, Resource, Serialize, Deserialize)]
pub enum MessagePayload {
	Bytes(Vec<u8>),
	Json(String),
	Dual(Vec<u8>, String),
}

impl MessagePayload {
	pub fn new<T: Serialize>(value: T) -> Result<Self> {
		let bytes = bincode::serialize(&value)?;
		#[cfg(not(feature = "serde_json"))]
		return Ok(Self::Bytes(bytes));
		#[cfg(feature = "serde_json")]
		{
			let json = serde_json::to_string(&value)?;
			return Ok(Self::Dual(bytes, json));
		}
	}

	pub fn into_bytes(&self) -> Result<Self> {
		match self {
			Self::Bytes(bytes) => Ok(Self::Bytes(bytes.clone())),
			Self::Json(_) => {
				anyhow::bail!(
					"message payload is json, cannot be converted to bytes"
				)
			}
			Self::Dual(bytes, _) => Ok(Self::Bytes(bytes.clone())),
		}
	}
	pub fn into_json(&self) -> Result<Self> {
		match self {
			Self::Bytes(_) => anyhow::bail!(
				"message payload is bytes, cannot be converted to json"
			),
			Self::Json(json) => Ok(Self::Json(json.clone())),
			Self::Dual(_, json) => Ok(Self::Json(json.clone())),
		}
	}

	// pub fn json<T: Serialize>(value: T) -> serde_json::Result<Self> {
	// 	let json = serde_json::to_string(&value)?;
	// 	Ok(Self::Json(json))
	// }
	pub fn deserialize<T: DeserializeOwned>(&self) -> Result<T> {
		match self {
			Self::Bytes(bytes) => Ok(bincode::deserialize(bytes)?),
			Self::Json(json) => {
				#[cfg(feature = "serde_json")]
				return Ok(serde_json::from_str(json)?);
				#[cfg(not(feature = "serde_json"))]
				anyhow::bail!("message payload is json but `serde_json` feature is not enabled")
			}
			Self::Dual(bytes, _) => Ok(bincode::deserialize(bytes)?),
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let payload = MessagePayload::new(7)?;
		// let value: i32 = payload.deserialize()?;
		// expect(value).to_be(7)?;

		let message = Message::SendEvent {
			reg_id: RegistrationId::new_with(0),
			payload,
		};

		expect(&message).to_be(&Message::SendEvent {
			reg_id: RegistrationId::new_with(0),
			payload: MessagePayload::Dual(vec![7, 0, 0, 0], "7".to_string()),
		})?;
		let message = message.with_json_payload()?;
		expect(&message).to_be(&Message::SendEvent {
			reg_id: RegistrationId::new_with(0),
			payload: MessagePayload::Json("7".to_string()),
		})?;

		expect(message.with_bytes_payload()).to_be_err_str(
			"message payload is json, cannot be converted to bytes",
		)?;



		Ok(())
	}
}
