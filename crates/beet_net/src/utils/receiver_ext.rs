#![allow(async_fn_in_trait)]
use crate::prelude::async_broadcastReceiverTExt;
use async_broadcast::Receiver;
use async_broadcast::RecvError;
use async_broadcast::TryRecvError;



#[extend::ext]
pub impl<T: Clone> Receiver<T> {
	/// Calls `recv_direct`, and if it returns [`RecvError::Overflowed`], calls itself again.
	async fn recv_direct_overflow_ok(&mut self) -> Result<T, RecvError> {
		loop {
			match self.recv_direct().await {
				Ok(message) => break Ok(message),
				Err(RecvError::Overflowed(_)) => {
					continue;
				}
				Err(other) => break Err(other),
			}
		}
	}
	fn try_recv_all(&mut self) -> Result<Vec<T>, TryRecvError> {
		let mut vec = Vec::new();
		loop {
			match self.try_recv_overflow_ok() {
				Ok(message) => vec.push(message),
				Err(TryRecvError::Empty) => break Ok(vec),
				Err(other) => break Err(other),
				// Err(TryRecvError::Closed) => break Err(TryRecvError::Closed),
				// Err(TryRecvError::Overflowed(val)) => {
				// 	break Err(TryRecvError::Overflowed(val))
				// }
			}
		}
	}
	/// Calls `try_recv`, and if it returns `TryRecvError::Overflowed`, calls itself again.
	fn try_recv_overflow_ok(&mut self) -> Result<T, TryRecvError> {
		loop {
			match self.try_recv() {
				Ok(message) => break Ok(message),
				Err(TryRecvError::Overflowed(_)) => continue,
				Err(other) => break Err(other),
			}
		}
	}
}

#[extend::ext]
pub impl<T: Clone> Receiver<Vec<T>> {
	/// Calls `flatten` on the result of `try_recv_all`.
	fn try_recv_all_flat(&mut self) -> Result<Vec<T>, TryRecvError> {
		let val = self.try_recv_all()?.into_iter().flatten().collect();
		Ok(val)
	}
}
