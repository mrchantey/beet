use crate::prelude::*;
use anyhow::Result;
use async_broadcast::Receiver;
use async_broadcast::Sender;


#[derive(Debug, Default, Clone)]
pub struct LoopbackBroadcast<T: 'static + Clone + Send + Sync> {
	pub recv: BroadcastChannel<T>,
	pub send: BroadcastChannel<T>,
}


impl<T: 'static + Clone + Send + Sync> LoopbackBroadcast<T> {
	/// Use this to populate the loopback
	pub fn tx(&self) -> Sender<T> { self.recv.send.clone() }
	/// Use this to listen to the loopback
	pub fn rx(&self) -> Receiver<T> { self.send.recv.clone() }

	/// Call this inside a [`loop`], it will wait asynchronously for the next value to be sent
	pub async fn update(&mut self) -> Result<()> {
		let value = self.recv.recv.recv_direct().await?;
		self.send.send.broadcast_direct(value).await?;
		Ok(())
	}
}
