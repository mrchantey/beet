use async_broadcast::TrySendError;
use beet_net::pubsub::BroadcastChannel;
use beet_net::utils::ReceiverTExt;
use sweet::*;

#[sweet_test]
pub fn async_broadcast() -> Result<()> {
	let (tx, mut rx) = async_broadcast::broadcast::<u8>(2);
	tx.try_broadcast(8)?;
	tx.try_broadcast(9)?;
	expect(rx.try_recv()?).to_be(8)?;
	expect(rx.try_recv()?).to_be(9)?;
	let (tx, _rx) = async_broadcast::broadcast::<u8>(1);
	tx.try_broadcast(8)?;
	expect(tx.try_broadcast(8)).to_be(Err(TrySendError::Full(8)))?;

	Ok(())
}
#[sweet_test]
pub fn broadcast_channel() -> Result<()> {
	let channel = BroadcastChannel::unbounded();
	expect(channel.push(1)?).to_be(None)?;
	expect(channel.push(2)?).to_be(None)?;
	expect(channel.push(3)?).to_be(None)?;

	let mut channel = BroadcastChannel::bounded(2);
	expect(channel.push(1)?).to_be(None)?;
	expect(channel.push(2)?).to_be(None)?;
	expect(channel.push(3)?).to_be(Some(1))?;

	expect(channel.recv.try_recv_overflow_ok()?).to_be(2)?;
	expect(channel.recv.try_recv_overflow_ok()?).to_be(3)?;

	Ok(())
}
