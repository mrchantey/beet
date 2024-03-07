use serde::Deserialize;
use serde::Serialize;
use strum_macros::Display;


#[derive(
	Serialize,
	Deserialize,
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	// Deref,
	// DerefMut
)]
pub struct Qos(pub Vec<QosPolicy>);
impl Qos {
	pub fn new(items: Vec<QosPolicy>) -> Self { Self(items) }

	pub fn history_bound(&self) -> Option<usize> {
		self.0.iter().find_map(|qos| match qos {
			QosPolicy::History(QosHistory::Bounded(bound)) => Some(*bound),
			_ => None,
		})
	}
}


#[derive(
	Serialize,
	Deserialize,
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Display,
)]
pub enum QosPolicy {
	History(QosHistory),
	/// TODO use async-broadcast
	Brodacast,
}

#[derive(
	Default,
	Serialize,
	Deserialize,
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Display,
)]
pub enum QosHistory {
	#[default]
	Unbounded,
	Bounded(usize),
}


impl Into<QosPolicy> for QosHistory {
	fn into(self) -> QosPolicy { QosPolicy::History(self) }
}
