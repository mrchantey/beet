#![feature(
	fn_traits,
	async_fn_traits,
	async_closure,
	unboxed_closures,
	never_type
)]
pub mod pubsub;
pub mod relay;
pub mod topic;
pub mod utils;


pub mod prelude {
	pub use crate::pubsub::*;
	pub use crate::relay::*;
	pub use crate::topic::*;
	pub use crate::utils::*;
	
	// temp fix until resolved https://github.com/smol-rs/async-broadcast/issues/50
	#[extend::ext]
	pub impl<T: Clone> async_broadcast::Sender<T> {
		fn broadcast_direct(&self, val: T) -> async_broadcast::Send<T> {
			self.broadcast(val)
		}
	}

	#[extend::ext]
	pub impl<T: Clone> async_broadcast::Receiver<T> {
		fn recv_direct(&mut self) -> async_broadcast::Recv<T> { self.recv() }
	}
}
