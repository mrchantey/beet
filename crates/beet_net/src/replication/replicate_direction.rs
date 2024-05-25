use serde::Deserialize;
use serde::Serialize;

#[derive(
	Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize,
)]
pub enum ReplicateDirection {
	#[default]
	Both,
	Incoming,
	Outgoing,
}


impl ReplicateDirection {
	pub fn is_incoming(&self) -> bool {
		match self {
			ReplicateDirection::Both => true,
			ReplicateDirection::Incoming => true,
			ReplicateDirection::Outgoing => false,
		}
	}

	pub fn is_outgoing(&self) -> bool {
		match self {
			ReplicateDirection::Both => true,
			ReplicateDirection::Incoming => false,
			ReplicateDirection::Outgoing => true,
		}
	}
}
