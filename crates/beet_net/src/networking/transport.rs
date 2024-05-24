// use anyhow::Result;
use crate::prelude::*;
use flume::Receiver;
use flume::Sender;
use std::sync::Arc;
use std::sync::Mutex;

pub trait Transport {
	#[allow(async_fn_in_trait)]
	async fn send_bytes(&mut self, bytes: Vec<u8>)
		-> Result<(), anyhow::Error>;
	fn recv_bytes(&mut self) -> Result<Vec<Vec<u8>>, anyhow::Error>;


	#[allow(async_fn_in_trait)]
	async fn send(
		&mut self,
		messages: &Vec<Message>,
	) -> Result<(), anyhow::Error> {
		self.send_bytes(Message::into_bytes(messages)?).await
	}


	fn recv(&mut self) -> Result<Vec<Message>, anyhow::Error> {
		let messages = self
			.recv_bytes()?
			.into_iter()
			.map(|b| Message::from_bytes(&b))
			.collect::<Result<Vec<_>, _>>()?
			.into_iter()
			.flatten()
			.collect::<Vec<_>>();

		Ok(messages)
	}
}

pub trait SendTransport: 'static + Clone + Send + Sync + Transport {}
impl<T: 'static + Clone + Send + Sync + Transport> SendTransport for T {}


impl<T: Transport> Transport for Arc<Mutex<T>> {
	async fn send_bytes(
		&mut self,
		bytes: Vec<u8>,
	) -> Result<(), anyhow::Error> {
		self.lock().unwrap().send_bytes(bytes).await
	}

	fn recv_bytes(&mut self) -> Result<Vec<Vec<u8>>, anyhow::Error> {
		self.lock().unwrap().recv_bytes()
	}
}

pub struct ChannelsTransport {
	pub send: Sender<Vec<u8>>,
	pub recv: Receiver<Vec<u8>>,
}

impl ChannelsTransport {
	pub fn loopback() -> Self {
		let (send, recv) = flume::unbounded();
		Self { send, recv }
	}

	pub fn new(send: Sender<Vec<u8>>, recv: Receiver<Vec<u8>>) -> Self {
		Self { send, recv }
	}

	pub fn pair() -> (Self, Self) {
		let (send1, recv1) = flume::unbounded();
		let (send2, recv2) = flume::unbounded();
		(Self::new(send1, recv2), Self::new(send2, recv1))
	}
}

impl Transport for ChannelsTransport {
	async fn send_bytes(
		&mut self,
		bytes: Vec<u8>,
	) -> Result<(), anyhow::Error> {
		self.send.send(bytes)?;
		Ok(())
	}

	fn recv_bytes(&mut self) -> Result<Vec<Vec<u8>>, anyhow::Error> {
		self.recv.try_recv_all()
	}
}


#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[tokio::test]
	async fn works() -> Result<()> {
		let (mut a, mut b) = ChannelsTransport::pair();

		a.send(&vec![Message::Spawn {
			entity: Entity::PLACEHOLDER,
		}])
		.await?;

		expect(b.recv()?).to_be(vec![Message::Spawn {
			entity: Entity::PLACEHOLDER,
		}])?;

		Ok(())
	}
}
