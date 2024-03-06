use crate::prelude::*;
use anyhow::Result;
use async_broadcast::Sender;
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

	/// Typesafe [`async_broadcast::Sender::broadcast`]
	pub async fn broadcast_pinned(&self, payload: &T) -> Result<MessageId> {
		let message_id = self.message_incr.next();
		let message =
			StateMessage::new(self.topic.clone(), payload, message_id)?;

		let _ = self.send.broadcast(message.clone()).await?;

		Ok(message_id)
	}
	/// Typesafe [`async_broadcast::Sender::broadcast_direct`]
	pub async fn broadcast(&self, payload: &T) -> Result<MessageId> {
		let message_id = self.message_incr.next();

		let message =
			StateMessage::new(self.topic.clone(), payload, message_id)?;

		let _ = self.send.broadcast_direct(message.clone()).await?;
		Ok(message_id)
	}
	// pub fn broadcast_blocking(&self, payload: &T) -> Result<MessageId> {
	// 	futures::executor::block_on(self.broadcast(payload))
	// }
}
