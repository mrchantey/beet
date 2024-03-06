use crate::prelude::*;
use anyhow::Result;
use std::sync::Arc;




impl Relay {
	/// Create a publisher for a topic
	pub fn add_publisher<T: Payload>(
		&self,
		topic: impl Into<Topic>,
	) -> Result<Publisher<T>> {
		self.add_publisher_with_type(topic, &dodgy_get_payload_type::<T>())
			.map(|val| val.recast())
	}

	/// Create a non-generic publisher for a topic
	pub(crate) fn add_publisher_with_type(
		&self,
		topic: impl Into<Topic>,
		payload_type: &PayloadType,
	) -> Result<Publisher<Vec<u8>>> {
		let topic = &topic.into();
		self.endpoint.add_publisher(topic, payload_type)?;

		let send = self.channel_map.get_or_insert_channel(topic).send;

		Ok(Publisher::new(
			topic.clone(),
			send,
			Arc::clone(&self.channel_map.message_id_incr),
		))
	}

	/// Create a subscriber for a topic
	pub fn add_subscriber<T: Payload>(
		&self,
		topic: impl Into<Topic>,
	) -> Result<Subscriber<T>> {
		self.add_subscriber_with_type(topic, &dodgy_get_payload_type::<T>())
			.map(|val| val.recast())
	}

	/// Create a non-generic subscriber for a topic
	pub(crate) fn add_subscriber_with_type(
		&self,
		topic: impl Into<Topic>,
		payload_type: &PayloadType,
	) -> Result<Subscriber<Vec<u8>>> {
		let topic = &topic.into();

		self.endpoint.add_subscriber(topic, payload_type)?;

		let recv = self.channel_map.get_or_insert_channel(topic).recv;

		Ok(Subscriber::new(topic.clone(), recv))
	}
}
