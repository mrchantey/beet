use anyhow::Result;
use flume::Receiver;
use flume::Sender;
use flume::TrySendError;

// absolutely arbitary at this point
// pub const DEFAULT_BROADCAST_CHANNEL_CAPACITY: usize = 128;


#[derive(Debug, Clone)]
pub struct BroadcastChannel<T> {
	pub send: Sender<T>,
	pub recv: Receiver<T>,
}


impl<T> Default for BroadcastChannel<T> {
	fn default() -> Self { Self::unbounded() }
}

impl<T> BroadcastChannel<T> {
	pub fn unbounded() -> Self {
		let (send, recv) = flume::unbounded();
		Self { send, recv }
	}

	pub fn bounded(capacity: usize) -> Self {
		let (send, recv) = flume::bounded(capacity);
		Self { send, recv }
	}
}
impl<T: 'static + Send + Sync> BroadcastChannel<T> {
	/// Push a message to the channel, returning the message that was popped if the channel was full
	pub fn push(&self, msg: T) -> Result<Option<T>> {
		match self.send.try_send(msg) {
			Ok(()) => Ok(None),
			Err(TrySendError::Full(unsent)) => {
				let popped = self.recv.recv()?;
				self.send.send(unsent)?;
				Ok(Some(popped))
			}
			Err(other) => Err(other.into()),
		}
	}
}
