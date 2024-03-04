use crate::prelude::*;
use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Debug;
use std::fmt::Display;

pub type PayloadType = String;
/// used to distinguish Request/Response messages
pub type MessageId = u64;



pub type StateMessageList = Vec<StateMessage>;


/// we need a better solution, this offers no guarantees
pub fn dodgy_get_payload_type<T>() -> PayloadType {
	std::any::type_name::<T>().to_string()
}
pub fn assert_payload_type(
	address: impl Display,
	expected: &PayloadType,
	received: &PayloadType,
) -> Result<()> {
	if received != expected {
		anyhow::bail!(
			"Type mismatch for {address}\nexpected {expected}, received {received}"		)
	}
	Ok(())
}


pub trait Payload:
	'static + Debug + Clone + Serialize + DeserializeOwned + Send + Sync
{
}
impl<T> Payload for T where
	T: 'static + Debug + Clone + Serialize + DeserializeOwned + Send + Sync
{
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateMessage {
	pub payload_type: PayloadType,
	pub topic: Topic,
	pub payload: Vec<u8>,
	pub id: MessageId,
}

impl Display for StateMessage {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"Message {{ topic: {}, payload_type: {}, payload_len: {} }}",
			self.topic,
			self.payload_type,
			self.payload.len()
		)
	}
}

impl StateMessage {
	pub fn new<T: Payload>(
		topic: Topic,
		payload: &T,
		id: MessageId,
	) -> Result<Self> {
		let payload_type = std::any::type_name::<T>().to_string();
		Ok(Self {
			payload_type,
			topic,
			id,
			payload: to_vec(payload)?,
		})
	}

	pub fn payload<T: Payload>(&self) -> Result<T> {
		assert_payload_type(
			&self.topic.address,
			&self.payload_type,
			&dodgy_get_payload_type::<T>(),
		)?;
		from_slice(&self.payload)
	}
}

pub(crate) fn to_vec(val: &impl Serialize) -> Result<Vec<u8>> {
	let out = bincode::serialize(val)?;
	Ok(out)
}

pub(crate) fn from_slice<T: DeserializeOwned>(val: &[u8]) -> Result<T> {
	let out = bincode::deserialize(val)?;
	Ok(out)
}

impl Into<StateMessageList> for StateMessage {
	fn into(self) -> StateMessageList { vec![self] }
}



// fn from_slice_with_buffer_size<
// 	const BUFFER_SIZE: usize,
// 	T: DeserializeOwned,
// >(
// 	val: &[u8],
// ) -> Result<T> {
// 	let mut buff = [0u8; BUFFER_SIZE];
// 	let out = ciborium::de::from_reader_with_buffer(val, &mut buff)?;
// 	Ok(out)
// }
// fn from_slice_with_heap_buffer_size<
// 	const BUFFER_SIZE: usize,
// 	T: DeserializeOwned,
// >(
// 	val: &[u8],
// ) -> Result<T> {
// 	let mut buff = Box::new([0u8; BUFFER_SIZE]);
// 	let out = ciborium::de::from_reader_with_buffer(val, &mut *buff)?;
// 	Ok(out)
// }
