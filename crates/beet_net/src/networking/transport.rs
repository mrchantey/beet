// use anyhow::Result;
use crate::prelude::*;
use flume::Receiver;
use flume::Sender;

pub trait Transport {
	fn send(&mut self, messages: &Vec<Message>) -> Result<(), anyhow::Error>;
	fn recv(&mut self) -> Result<Vec<Message>, anyhow::Error>;
}

pub struct ChannelsTransport {
	pub send: Sender<Vec<Message>>,
	pub recv: Receiver<Vec<Message>>,
}

impl ChannelsTransport {
	pub fn loopback() -> Self {
		let (send, recv) = flume::unbounded();
		Self { send, recv }
	}

	pub fn new(
		send: Sender<Vec<Message>>,
		recv: Receiver<Vec<Message>>,
	) -> Self {
		Self { send, recv }
	}

	pub fn pair() -> (Self, Self) {
		let (send1, recv1) = flume::unbounded();
		let (send2, recv2) = flume::unbounded();
		(Self::new(send1, recv2), Self::new(send2, recv1))
	}
}

impl Transport for ChannelsTransport {
	fn send(&mut self, messages: &Vec<Message>) -> Result<(), anyhow::Error> {
		self.send.send(messages.clone())?;
		Ok(())
	}

	fn recv(&mut self) -> Result<Vec<Message>, anyhow::Error> {
		self.recv.try_recv_all_flat()
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
		}])?;

		expect(b.recv()?).to_be(vec![Message::Spawn {
			entity: Entity::PLACEHOLDER,
		}])?;

		Ok(())
	}
}
