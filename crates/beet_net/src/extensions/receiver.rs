use flume::Receiver;
use flume::TryRecvError;


#[extend::ext(name=FlumeReceiverExt)]
pub impl<T> Receiver<T> {
	fn try_recv_all(&mut self) -> anyhow::Result<Vec<T>> {
		let mut vec = Vec::new();
		loop {
			match self.try_recv() {
				Ok(message) => vec.push(message),
				Err(TryRecvError::Empty) => break Ok(vec),
				Err(other) => anyhow::bail!(other),
			}
		}
	}
}
#[extend::ext(name=FlumeReceiverExtFlat)]
pub impl<T> Receiver<Vec<T>> {
	fn try_recv_all_flat(&mut self) -> anyhow::Result<Vec<T>> {
		let mut vec = Vec::new();
		loop {
			match self.try_recv() {
				Ok(message) => vec.extend(message),
				Err(TryRecvError::Empty) => break Ok(vec),
				Err(other) => anyhow::bail!(other),
			}
		}
	}
}
