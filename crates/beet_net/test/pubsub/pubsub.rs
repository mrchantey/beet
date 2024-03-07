use beet_net::prelude::*;
use sweet::*;

#[sweet_test(non_send)]
pub fn pubsub_fail_cases() -> Result<()> {
	let relay = Relay::default();
	let topic = Topic::pubsub_update("foo/bar");
	let _sub = relay.add_subscriber_with_topic::<u8>(&topic)?;
	let bad_sub = relay.add_subscriber_with_topic::<u32>(&topic);

	let err_str =
		"Type mismatch for PubSub.Update/foo/bar:0\nexpected u8, received u32";
	expect(bad_sub).to_be_err_str(err_str)?;
	let bad_pub = relay.add_publisher_with_topic::<u32>(&topic);
	expect(bad_pub).to_be_err_str(err_str)?;
	Ok(())
}




#[sweet_test(non_send)]
pub async fn pubsub() -> Result<()> {
	let relay = Relay::default();
	let topic = Topic::pubsub_update("foo/bar");
	let mut sub1 = relay.add_subscriber_with_topic::<u8>(&topic)?;
	// let mut sub2 = relay.add_subscriber::<u8>(&topic)?;
	let publisher = relay.add_publisher_with_topic::<u8>(&topic)?;
	publisher.send(&8_u8)?;
	let out1 = sub1.recv()?;
	// let out2 = sub2.recv()?;
	expect(out1).to_be(8_u8)?;
	// expect(out2).to_be(8_u8)?;
	Ok(())
}

#[sweet_test(non_send, skip)]
pub async fn async_broadcast() -> Result<()> {
	// let (tx, rx) = async_broadcast::broadcast(1);

	// let mut rx1 = rx.clone();
	// let mut rx2 = rx.clone();
	// tx.broadcast(8).await?;
	// expect(rx1.recv_direct().await?).to_be(8)?;
	// expect(rx2.recv_direct().await?).to_be(8)?;

	Ok(())
}

#[sweet_test(non_send, skip)]
pub async fn broadcast() -> Result<()> {
	let relay = Relay::default();
	let topic = Topic::pubsub_update("foo/bar");
	let mut sub1 = relay.add_subscriber_with_topic::<u8>(&topic)?;
	let mut sub2 = relay.add_subscriber_with_topic::<u8>(&topic)?;
	let publisher = relay.add_publisher_with_topic::<u8>(&topic)?;
	publisher.send(&8_u8)?;
	let out1 = sub1.recv()?;
	let out2 = sub2.recv()?;
	expect(out1).to_be(8_u8)?;
	expect(out2).to_be(8_u8)?;
	Ok(())
}
