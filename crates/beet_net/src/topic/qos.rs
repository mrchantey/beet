use serde::Deserialize;
use serde::Serialize;
use strum_macros::Display;


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
pub enum Qos {
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
	Bounded(u64),
}
