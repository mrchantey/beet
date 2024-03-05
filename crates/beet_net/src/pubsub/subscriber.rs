use crate::prelude::*;
use anyhow::Result;
use async_broadcast::Receiver;
use std::marker::PhantomData;
/// Typesafe wrapper for [`async_broadcast::Receiver`] for a specific topic.
/// Held by the subscriber to listen for messages from the relay
#[derive(Debug, Clone)]
pub struct Subscriber<T: Payload> {
	topic: Topic,
	recv: Receiver<StateMessage>,
	phantom: PhantomData<T>,
}

impl<T: Payload> Subscriber<T> {
	pub fn new(topic: Topic, recv: Receiver<StateMessage>) -> Self {
		Self {
			topic,
			recv,
			phantom: PhantomData,
		}
	}
	pub fn recast<U: Payload>(self) -> Subscriber<U> {
		let Subscriber { topic, recv, .. } = self;

		Subscriber {
			topic,
			recv,
			phantom: PhantomData,
		}
	}
	pub fn topic(&self) -> &Topic { &self.topic }
	pub fn recv_inner(&self) -> &Receiver<StateMessage> { &self.recv }
	pub fn recv_inner_mut(&mut self) -> &mut Receiver<StateMessage> {
		&mut self.recv
	}

	/// Typesafe [`async_broadcast::Receiver::recv`]
	pub async fn recv_pinned(&mut self) -> Result<T> {
		Ok(self.recv.recv().await?.payload()?)
	}

	/// Typesafe [`async_broadcast::Receiver::recv_direct`]
	pub async fn recv(&mut self) -> Result<T> {
		Ok(self.recv.recv_direct().await?.payload()?)
	}
	#[cfg(feature = "tokio")]
	pub async fn recv_timeout(
		&mut self,
		timeout: std::time::Duration,
	) -> Result<T> {
		Ok(tokio::time::timeout(timeout, self.recv()).await??)
	}
	#[cfg(feature = "tokio")]
	pub async fn recv_default_timeout(&mut self) -> Result<T> {
		Ok(
			tokio::time::timeout(
				std::time::Duration::from_secs(1),
				self.recv(),
			)
			.await??,
		)
	}

	pub fn try_recv_all(&mut self) -> Result<Vec<T>> {
		let vec = self
			.recv
			.try_recv_all()?
			.into_iter()
			.map(|message| message.payload())
			.collect::<Result<Vec<_>>>()?;
		Ok(vec)
	}
	pub fn try_recv_all_messages(&mut self) -> Result<Vec<StateMessage>> {
		let vec = self.recv.try_recv_all()?.into_iter().collect();
		Ok(vec)
	}
}
