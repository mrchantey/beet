use crate::prelude::*;
use anyhow::Result;
use async_broadcast::Receiver;
use bevy_utils::HashMap;
use std::sync::Arc;
use std::sync::RwLock;


pub type TopicChannel = BroadcastChannel<StateMessage>;


/// A channel map stores a map of topics and their associated broadcast channels
/// It has no sense of payload type, and is used to create publishers and subscribers
#[derive(Debug, Default, Clone)]
pub struct TopicChannelMap {
	map: Arc<RwLock<HashMap<Topic, TopicChannel>>>,
	pub(crate) message_id_incr: Arc<IdIncr>,
}

pub const POISONED_LOCK: &str = "Poisoned lock";

impl TopicChannelMap {
	pub fn new_with_on_change() -> Self {
		let channel_map = Self::default();
		channel_map.map.write().expect(POISONED_LOCK).insert(
			TopicSetEndpoint::on_change_topic(),
			TopicChannel::default(),
		);
		channel_map
	}


	pub(crate) fn get_or_insert_channel(&self, topic: &Topic) -> TopicChannel {
		{
			if let Some(channels) =
				self.map.read().expect(POISONED_LOCK).get(topic)
			{
				return channels.clone();
			}
		}
		let channels = TopicChannel::default();
		self.map
			.write()
			.expect(POISONED_LOCK)
			.insert(topic.clone(), channels.clone());
		channels
	}

	/// Create a publisher without checking the payload type
	pub(crate) fn unchecked_publisher<T: Payload>(
		&self,
		address: TopicAddress,
		method: TopicMethod,
	) -> Publisher<T> {
		let topic = Topic::new(address, TopicScheme::PubSub, method);
		let channel = self.get_or_insert_channel(&topic);

		let publisher = Publisher::new(
			topic.clone(),
			channel.send.clone(),
			self.message_id_incr.clone(),
		);

		publisher
	}
	/// Create a subscriber without checking the payload type
	pub fn unchecked_subscriber<T: Payload>(
		&self,
		address: TopicAddress,
		method: TopicMethod,
	) -> Subscriber<T> {
		let topic = Topic::new(address, TopicScheme::PubSub, method);
		let channel = self.get_or_insert_channel(&topic);
		Subscriber::new(topic.clone(), channel.recv.clone())
	}



	pub(crate) fn try_get_channel(
		&self,
		topic: &Topic,
	) -> Option<TopicChannel> {
		self.map.read().expect(POISONED_LOCK).get(topic).cloned()
	}
	// fn try_get_channel_ok(&self, topic: &Topic) -> Result<TopicChannel> {
	// 	self.try_get_channel(topic)
	// 		.ok_or(anyhow::anyhow!("Topic not found"))
	// }

	pub async fn try_broadcast(&self, message: StateMessage) -> Result<bool> {
		let topic = message.topic.clone();
		if let Some(channel) = self.try_get_channel(&topic) {
			channel.send.broadcast(message).await?;
			Ok(true)
		} else {
			Ok(false)
		}
	}

	/// Get all published messages ready to be sent
	fn get_all_recv(&self) -> Vec<Receiver<StateMessage>> {
		self.map
			.read()
			.expect(POISONED_LOCK)
			.values()
			.map(|c| c.recv.clone())
			.collect()
	}

	pub fn get_all_messages(&mut self) -> Result<Vec<StateMessage>> {
		let messages = self
			.get_all_recv()
			.into_iter()
			.map(|mut recv| recv.try_recv_all())
			.collect::<Result<Vec<_>, _>>()?
			.into_iter()
			.flatten()
			.collect::<Vec<_>>();
		Ok(messages)
	}

	/// Try and send all messages to their
	/// # Returns
	/// A list of messages that could not be sent
	pub async fn try_send_all_messages(
		&self,
		messages: Vec<StateMessage>,
	) -> Result<Vec<StateMessage>> {
		let futs = messages.into_iter().map(|message| {
			let channel = self.try_get_channel(&message.topic);
			async {
				let result: Result<_> = if let Some(channel) = channel {
					channel.send.broadcast_direct(message).await?;
					Ok(None)
				} else {
					Ok(Some(message))
				};
				result
			}
		});
		let out = futures::future::try_join_all(futs).await?;
		// filter map
		let out = out.into_iter().filter_map(|o| o).collect();

		Ok(out)
	}
}


pub struct ChannelNotFoundError {
	topic: Topic,
}
impl std::fmt::Debug for ChannelNotFoundError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "ChannelNotFoundError: {:?}", self.topic)
	}
}
impl std::fmt::Display for ChannelNotFoundError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "ChannelNotFoundError: {:?}", self.topic)
	}
}
impl std::error::Error for ChannelNotFoundError {}
