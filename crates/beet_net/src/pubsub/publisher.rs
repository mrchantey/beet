use crate::prelude::*;
use anyhow::Result;
use std::marker::PhantomData;
use std::sync::Arc;

/// Typesafe wrapper for [`async_broadcast::Sender`] for a specific topic.
/// Held by the publisher to notify relay of a message
#[derive(Debug, Clone)]
pub struct Publisher<T: Payload> {
	topic: Topic,
	channel: BroadcastChannel<StateMessage>,
	message_incr: Arc<IdIncr>,
	phantom: PhantomData<T>,
}

impl<T: Payload> Publisher<T> {
	pub fn new(
		topic: Topic,
		channel: BroadcastChannel<StateMessage>,
		message_incr: Arc<IdIncr>,
	) -> Self {
		Self {
			topic,
			channel,
			message_incr,
			phantom: PhantomData,
		}
	}

	pub fn recast<U: Payload>(self) -> Publisher<U> {
		let Publisher {
			topic,
			channel,
			message_incr,
			..
		} = self;
		Publisher {
			topic,
			channel,
			message_incr,
			phantom: PhantomData,
		}
	}
	pub fn topic(&self) -> &Topic { &self.topic }
	pub fn channel_inner(&self) -> &BroadcastChannel<StateMessage> {
		&self.channel
	}


	fn into_message(&self, payload: &T) -> Result<StateMessage> {
		let message_id = self.message_incr.next();
		StateMessage::new(self.topic.clone(), payload, message_id)
	}

	pub fn push(&self, payload: &T) -> Result<MessageId> {
		let message = self.into_message(payload)?;
		let id = message.id;
		self.channel.push(message)?;
		Ok(id)
	}
}
