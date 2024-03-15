use crate::prelude::*;
use anyhow::Result;
use bevy::utils::HashMap;
use bevy::utils::HashSet;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;


/// A representation of all topic interests (sub) and intents (pub)
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TopicSet {
	subscribers: HashMap<Topic, PayloadType>,
	publishers: HashMap<Topic, PayloadType>,
}
impl TopicSet {
	pub fn publishers(&self) -> &HashMap<Topic, PayloadType> {
		&self.publishers
	}
	pub fn subscribers(&self) -> &HashMap<Topic, PayloadType> {
		&self.subscribers
	}

	pub fn extend(&mut self, other: TopicSet) {
		self.subscribers.extend(other.subscribers);
		self.publishers.extend(other.publishers);
	}
	pub fn extend_all(&mut self, others: impl IntoIterator<Item = TopicSet>) {
		for other in others {
			self.extend(other);
		}
	}

	pub fn topics(&self) -> HashSet<Topic> {
		self.subscribers
			.keys()
			.chain(self.publishers.keys())
			.cloned()
			.collect()
	}

	pub fn try_add_publisher(
		&mut self,
		topic: &Topic,
		payload_type: &PayloadType,
	) -> Result<bool> {
		self.check_payload_types(topic, payload_type)?;
		let did_insert = self
			.publishers
			.insert(topic.clone(), payload_type.clone())
			.is_none();
		Ok(did_insert)
	}
	pub fn try_add_subscriber(
		&mut self,
		topic: &Topic,
		payload_type: &PayloadType,
	) -> Result<bool> {
		self.check_payload_types(topic, payload_type)?;
		let did_insert = self
			.subscribers
			.insert(topic.clone(), payload_type.clone())
			.is_none();
		Ok(did_insert)
	}


	pub fn has_publisher(
		&self,
		topic: &Topic,
		payload_type: &PayloadType,
	) -> Result<bool> {
		self.check_payload_types(topic, payload_type)?;
		Ok(self.publishers.contains_key(topic))
	}
	pub fn has_subscriber(
		&self,
		topic: &Topic,
		payload_type: &PayloadType,
	) -> Result<bool> {
		self.check_payload_types(topic, payload_type)?;
		Ok(self.subscribers.contains_key(topic))
	}

	pub fn payloads(&self) -> HashMap<Topic, PayloadType> {
		self.subscribers
			.iter()
			.chain(self.publishers.iter())
			.map(|(topic, payload_type)| (topic.clone(), payload_type.clone()))
			.collect()
	}


	fn check_payload_types(
		&self,
		topic: &Topic,
		payload_type: &PayloadType,
	) -> Result<()> {
		if let Some(expected) = self.publishers.get(topic) {
			assert_payload_type(topic, expected, payload_type)?;
		}
		if let Some(expected) = self.subscribers.get(topic) {
			assert_payload_type(topic, expected, payload_type)?;
		}
		Ok(())
	}
}

impl fmt::Display for TopicSet {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let payloads = self
			.payloads()
			.into_iter()
			.map(|(topic, payload)| format!("{topic}:{payload}"))
			.collect::<Vec<_>>()
			.join("\n");

		write!(f, "Payloads:\n{}", payloads)
	}
}
