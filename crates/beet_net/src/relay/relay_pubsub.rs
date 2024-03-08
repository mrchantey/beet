use crate::prelude::*;
use anyhow::Result;
use std::sync::Arc;




impl Relay {
	/// send once without keeping a publisher
	pub fn send<T: Payload>(
		&self,
		topic: impl Into<Topic>,
		payload: &T,
	) -> Result<MessageId> {
		self.add_publisher_with_topic(topic)?.push(payload)
	}

	/// recv once without keeping a subscriber
	pub fn recv<T: Payload>(&self, topic: impl Into<Topic>) -> Result<T> {
		self.add_subscriber_with_topic(topic)?.try_recv()
	}




	/// Create a publisher for a topic
	pub fn add_publisher<T: Payload>(
		&self,
		address: impl Into<TopicAddress>,
		method: TopicMethod,
	) -> Result<Publisher<T>> {
		self.add_publisher_with_topic(Topic::new(
			address,
			TopicScheme::PubSub,
			method,
		))
	}
	pub fn add_publisher_with_topic<T: Payload>(
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

		let channel = self.channel_map.get_or_insert_channel(topic);

		Ok(Publisher::new(
			topic.clone(),
			channel,
			Arc::clone(&self.channel_map.message_id_incr),
		))
	}

	/// Create a subscriber for a topic
	pub fn add_subscriber<T: Payload>(
		&self,
		address: impl Into<TopicAddress>,
		method: TopicMethod,
	) -> Result<Subscriber<T>> {
		self.add_subscriber_with_topic(Topic::new(
			address,
			TopicScheme::PubSub,
			method,
		))
	}
	/// Create a subscriber for a topic
	pub fn add_subscriber_with_topic<T: Payload>(
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
