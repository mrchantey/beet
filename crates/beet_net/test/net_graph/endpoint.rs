// // use bevy_utils::Duration;
use beet_net::prelude::*;
use sweet::*;

#[sweet_test(non_send)]
pub async fn calls_topic_added() -> Result<()> {
	let topic = Topic::pubsub_update("foo/bar");
	let relay = Relay::default();

	let mut on_change = relay.topic_set_changed();
	expect(on_change.try_recv_all()?.len()).to_be(0)?;
	let _some_pub = relay.add_publisher::<u8>(&topic).await?;
	expect(on_change.try_recv_all()?.len()).to_be(1)?;

	Ok(())
}
