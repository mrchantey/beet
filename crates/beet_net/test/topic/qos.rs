use beet_net::pubsub::BroadcastChannel;
use flume::TrySendError;
use sweet::*;

#[sweet_test]
pub fn flume() -> Result<()> {
	let (tx, rx) = flume::unbounded::<u8>();
	tx.send(8)?;
	tx.send(9)?;
	expect(rx.recv()?).to_be(8)?;
	expect(rx.recv()?).to_be(9)?;
	let (tx, _rx) = flume::bounded::<u8>(1);
	tx.try_send(8)?;
	expect(tx.try_send(8)).to_be(Err(TrySendError::Full(8)))?;

	Ok(())
}
#[sweet_test]
pub fn broadcast_channel() -> Result<()> {
	let channel = BroadcastChannel::unbounded();
	expect(channel.push(1)?).to_be(None)?;
	expect(channel.push(2)?).to_be(None)?;
	expect(channel.push(3)?).to_be(None)?;

	let channel = BroadcastChannel::bounded(2);
	expect(channel.push(1)?).to_be(None)?;
	expect(channel.push(2)?).to_be(None)?;
	expect(channel.push(3)?).to_be(Some(1))?;

	expect(channel.recv.recv()?).to_be(2)?;
	expect(channel.recv.recv()?).to_be(3)?;

	Ok(())
}
