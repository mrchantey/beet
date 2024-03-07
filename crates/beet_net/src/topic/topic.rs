use super::*;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Display;
use strum_macros::Display;


pub type TopicKey = u64;
pub type TopicPath = String;


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


#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	// Deref,
	// DerefMut,
	Serialize,
	Deserialize,
)]
pub struct TopicAddress {
	domain: TopicDomain,
	path: TopicPath,
	key: TopicKey,
}

impl Display for TopicAddress {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}", self.path, self.key)
	}
}

impl<T> From<T> for TopicAddress
where
	T: Into<TopicPath>,
{
	fn from(path: T) -> Self { TopicAddress::new(path) }
	// fn into(self) -> TopicAddress { TopicAddress::new(self) }
}

impl Into<TopicAddress> for &TopicAddress {
	fn into(self) -> TopicAddress { self.clone() }
}

impl TopicAddress {
	// pub fn new(topic: impl ToString) -> Topic { Topic(topic.to_string()) }
	pub fn new(path: impl Into<TopicPath>) -> TopicAddress {
		TopicAddress {
			path: path.into(),
			key: 0,
			domain: TopicDomain::default(),
		}
	}
	pub fn new_with_key(
		topic: impl Into<TopicPath>,
		key: TopicKey,
	) -> TopicAddress {
		TopicAddress {
			path: topic.into(),
			key,
			domain: TopicDomain::default(),
		}
	}
	pub fn new_with_key_and_domain(
		topic: impl Into<TopicPath>,
		key: TopicKey,
		domain: TopicDomain,
	) -> TopicAddress {
		TopicAddress {
			path: topic.into(),
			key,
			domain,
		}
	}

	pub fn push(&mut self, topic: &str) {
		self.path.push_str("/");
		self.path.push_str(topic);
	}

	pub fn pop(&mut self) {
		let mut v: Vec<&str> = self.path.split("/").collect();
		v.pop();
		self.path = v.join("/");
	}

	pub fn domain(&self) -> TopicDomain { self.domain.clone() }
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
	pub address: TopicAddress,
	pub scheme: TopicScheme,
	pub method: TopicMethod,
	pub qos: Vec<Qos>,
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

	pub fn pubsub_update(address: impl Into<TopicAddress>) -> Self {
		Self {
			address: address.into(),
			scheme: TopicScheme::PubSub,
			method: TopicMethod::Update,
			qos: Default::default(),
		}
	}

	pub fn with_qos(mut self, qos: Qos) -> Self {
		self.qos.push(qos);
		self
	}
}


impl Into<Topic> for &Topic {
	fn into(self) -> Topic { self.clone() }
}

// #[derive(Debug, Clone)]
// pub struct TopicChannel {
// 	pub payload_type: PayloadType,
// 	pub send: Sender<StateMessage>,
// 	pub recv: Receiver<StateMessage>,
// }



// impl TopicChannel {
// 	pub fn new_generic<T: Payload>() -> Self {
// 		Self::new(dodgy_get_payload_type::<T>())
// 	}
// 	pub fn new(payload_type: PayloadType) -> Self {
// 		let (send, recv) =
// 			async_broadcast::broadcast(DEFAULT_BROADCAST_CHANNEL_CAPACITY);
// 		Self {
// 			payload_type,
// 			send,
// 			recv,
// 		}
// 	}
// 	pub fn payload_type(&self) -> &PayloadType { &self.payload_type }
// }
