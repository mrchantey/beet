use crate::prelude::*;
use anyhow::Result;
use async_broadcast::Receiver;
use std::marker::PhantomData;
use std::time::Duration;
/// Typesafe wrapper for [`async_broadcast::Receiver`] for a specific topic.
/// Held by the subscriber to listen for messages from the relay
#[derive(Debug, Clone)]
pub struct Subscriber<T: Payload> {
	topic: Topic,
	recv: Receiver<StateMessage>,
	phantom: PhantomData<T>,
}

pub const DEFAULT_RECV_TIMEOUT: Duration = Duration::from_secs(1);

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

	/// Typesafe [`async_broadcast::Receiver::try_recv_overflow_ok`]
	pub fn try_recv(&mut self) -> Result<T> {
		Ok(self.recv.try_recv_overflow_ok()?.payload()?)
	}
	/// Typesafe [`async_broadcast::Receiver::recv_direct`] ignoring overflows
	pub async fn recv_async(&mut self) -> Result<T> {
		Ok(self.recv.recv_direct_overflow_ok().await?.payload()?)
	}

	/// Typesafe [`flume::Receiver::recv_timeout`]
	// #[allow(unused)]
	// pub fn recv_timeout(&mut self, timeout: std::time::Duration) -> Result<T> {
	// 	#[cfg(target_arch = "wasm32")]
	// 	todo!();
	// 	Ok(self.recv.recv_timeout(timeout)?.payload()?)
	// }
	// /// Typesafe [`flume::Receiver::recv_timeout`]
	// #[allow(unused)]
	// pub fn recv_default_timeout(&mut self) -> Result<T> {
	// 	#[cfg(target_arch = "wasm32")]
	// 	todo!();
	// 	Ok(self.recv.recv_timeout(DEFAULT_RECV_TIMEOUT)?.payload()?)
	// }
	// /// Typesafe [`flume::Receiver::recv_deadline`]
	// #[allow(unused)]
	// pub fn recv_deadline(&mut self, deadline: std::time::Instant) -> Result<T> {
	// 	#[cfg(target_arch = "wasm32")]
	// 	todo!();
	// 	Ok(self.recv.recv_deadline(deadline)?.payload()?)
	// }
	/// Typesafe [`flume::Receiver::try_recv`]

	/// Typesafe [`flume::Receiver::try_recv_all`]
	pub fn try_recv_all(&mut self) -> Result<Vec<T>> {
		let vec = self
			.recv
			.try_recv_all()?
			.into_iter()
			.map(|message| message.payload())
			.collect::<Result<Vec<_>>>()?;
		Ok(vec)
	}
}
