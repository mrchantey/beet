use async_broadcast::Receiver;
use async_broadcast::TryRecvError;
use std::error::Error;
use std::fmt;



#[extend::ext]
pub impl<T: Clone> Receiver<T> {
	fn try_recv_all(&mut self) -> Result<Vec<T>, TryRecvError> {
		let mut vec = Vec::new();
		loop {
			match self.try_recv() {
				Ok(message) => vec.push(message),
				Err(TryRecvError::Empty) => break Ok(vec),
				Err(other) => break Err(other), // Err(TryRecvError::Overflowed(val)) => {
				                                // 	break Err(TryRecvAllError::Overflowed(val))
				                                // }
			}
		}
	}
	/// Calls `try_recv`, and if it returns `TryRecvError::Overflowed`, calls itself again.
	fn try_recv_overflow_ok(&mut self) -> Result<T, TryRecvError> {
		loop {
			match self.try_recv() {
				Ok(message) => break Ok(message),
				Err(TryRecvError::Overflowed(_)) => {
					break self.try_recv_overflow_ok()
				}
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




/// An error returned from [`Receiver::try_recv()`].
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TryRecvAllError {
	/// The channel has overflowed since the last element was seen.  Future recv operations will
	/// succeed, but some messages have been skipped.
	Overflowed(u64),
	/// The channel is empty and closed.
	Disconnected,
}
impl Error for TryRecvAllError {}

impl fmt::Display for TryRecvAllError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match *self {
			TryRecvAllError::Disconnected => {
				write!(f, "receiving from an empty and disconnected channel")
			}
			TryRecvAllError::Overflowed(n) => {
				write!(f, "receiving operation observed {} lost messages", n)
			}
		}
	}
}