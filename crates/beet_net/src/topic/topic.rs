use super::*;
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
/// Description of the communication method.
pub enum TopicScheme {
	/// For an app to announce its changes in state
	PubSub,
	/// Used by Requesters and Responders, will set the state of an app
	Request,
	/// Used by Requesters and Responders, will return the result of the request.
	/// This may be an error.
	Response,
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
	Copy,
	Hash,
	Display,
)]
/// Description of the quality of topic state.
pub enum TopicMethod {
	/// Implementers are allowed to fail this if there is already an existing state
	Create,
	/// Implementers are allowed to fail this if there is no existing state
	Update,
	/// Implementers are allowed to fail this if there is no existing state
	Delete,
}

#[derive(
	Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct Topic {
	// domain: TopicDomain,
	pub address: TopicAddress,
	pub scheme: TopicScheme,
	pub method: TopicMethod,
	pub qos: Qos,
}

impl std::fmt::Display for Topic {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}.{}/{}", self.scheme, self.method, self.address)
	}
}

impl Topic {
	pub fn new(
		address: impl Into<TopicAddress>,
		scheme: TopicScheme,
		method: TopicMethod,
	) -> Self {
		Self {
			address: address.into(),
			scheme,
			method,
			qos: Default::default(),
		}
	}

	/// Create a topic with [`TopicScheme::PubSub`], [`TopicMethod::Create`], and [`QosHistory::Bounded`] with capacity of 1.
	/// The channel will store at most one message.
	pub fn pubsub_update(address: impl Into<TopicAddress>) -> Self {
		Self {
			address: address.into(),
			scheme: TopicScheme::PubSub,
			method: TopicMethod::Update,
			qos: Qos::new(vec![QosHistory::Bounded(1).into()]),
		}
	}

	pub fn with_qos(mut self, qos: QosPolicy) -> Self {
		self.qos.0.push(qos);
		self
	}

	// pub fn domain(&self) -> TopicDomain { self.domain.clone() }
}


impl Into<Topic> for &Topic {
	fn into(self) -> Topic { self.clone() }
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
pub enum TopicDomain {
	// Local,//hopefully dont need this
	Global(u64),
}

impl Default for TopicDomain {
	fn default() -> Self { TopicDomain::Global(1) }
}
