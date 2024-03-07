use crate::prelude::*;
use anyhow::Result;
use flume::Sender;
use std::marker::PhantomData;
use std::sync::Arc;

/// Typesafe wrapper for [`async_broadcast::Sender`] for a specific topic.
/// Held by the publisher to notify relay of a message
#[derive(Debug, Clone)]
pub struct Publisher<T: Payload> {
	topic: Topic,
	send: Sender<StateMessage>,
	message_incr: Arc<IdIncr>,
	phantom: PhantomData<T>,
}

impl<T: Payload> Publisher<T> {
	pub fn new(
		topic: Topic,
		send: Sender<StateMessage>,
		message_incr: Arc<IdIncr>,
	) -> Self {
		Self {
			topic,
			send,
			message_incr,
			phantom: PhantomData,
		}
	}

	pub fn recast<U: Payload>(self) -> Publisher<U> {
		let Publisher {
			topic,
			send,
			message_incr,
			..
		} = self;
		Publisher {
			topic,
			send,
			message_incr,
			phantom: PhantomData,
		}
	}
	pub fn topic(&self) -> &Topic { &self.topic }
	pub fn send_inner(&self) -> &Sender<StateMessage> { &self.send }


	fn into_message(&self, payload: &T) -> Result<StateMessage> {
		let message_id = self.message_incr.next();
		StateMessage::new(self.topic.clone(), payload, message_id)
	}

	/// Typesafe [`flume::Sender::send`]
	pub fn send(&self, payload: &T) -> Result<MessageId> {
		let message = self.into_message(payload)?;
		let id = message.id;
		self.send.send(message)?;
		Ok(id)
	}
}
