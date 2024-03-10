use async_broadcast::Receiver;
use async_broadcast::Sender;
use async_broadcast::TrySendError;

// absolutely arbitary at this point
pub const MAX_BROADCAST_CHANNEL_CAPACITY: usize = 2;


#[derive(Debug, Clone)]
pub struct BroadcastChannel<T> {
	pub send: Sender<T>,
	pub recv: Receiver<T>,
}


impl<T> Default for BroadcastChannel<T> {
	fn default() -> Self { Self::unbounded() }
}

impl<T> BroadcastChannel<T> {
	pub fn unbounded() -> Self { Self::bounded(MAX_BROADCAST_CHANNEL_CAPACITY) }

	pub fn bounded(capacity: usize) -> Self {
		let (mut send, mut recv) = async_broadcast::broadcast(capacity);
		send.set_overflow(true);
		recv.set_overflow(true);
		Self { send, recv }
	}
}
impl<T: 'static + Send + Sync + Clone> BroadcastChannel<T> {
	pub fn push(&self, msg: T) -> Result<Option<T>, TrySendError<T>> {
		self.send.try_broadcast(msg)
	}
}

// only needed for flume, async_broadcast has overflow builtin
// impl<T: 'static + Send + Sync + Clone> BroadcastChannel<T> {
// 	/// Push a message to the channel, returning the message that was popped if the channel was full
// 	pub fn push(&self, msg: T) -> Result<Option<T>> {
// 		match self.send.try_broadcast(msg) {
// 			Ok(()) => Ok(None),
// 			Err(TrySendError::Full(unsent)) => {
// 				let popped = self.recv.recv()?;
// 				self.send.send(unsent)?;
// 				Ok(Some(popped))
// 			}
// 			Err(other) => Err(other.into()),
// 		}
// 	}
// }
