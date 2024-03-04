use async_broadcast::Receiver;
use async_broadcast::Sender;

pub const DEFAULT_BROADCAST_CHANNEL_CAPACITY: usize = 4;


#[derive(Debug, Clone)]
pub struct BroadcastChannel<T> {
	pub(crate) send: Sender<T>,
	pub(crate) recv: Receiver<T>,
}


impl<T> Default for BroadcastChannel<T> {
	fn default() -> Self {
		let (send, recv) =
			async_broadcast::broadcast(DEFAULT_BROADCAST_CHANNEL_CAPACITY);
		Self { send, recv }
	}
}

impl<T> BroadcastChannel<T> {
	pub fn new(capacity: usize) -> Self {
		let (send, recv) = async_broadcast::broadcast(capacity);
		Self { send, recv }
	}
}
