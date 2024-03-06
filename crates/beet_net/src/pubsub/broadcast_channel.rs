use flume::Receiver;
use flume::Sender;

// absolutely arbitary at this point
// pub const DEFAULT_BROADCAST_CHANNEL_CAPACITY: usize = 128;


#[derive(Debug, Clone)]
pub struct BroadcastChannel<T> {
	pub(crate) send: Sender<T>,
	pub(crate) recv: Receiver<T>,
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
