use beet_net::prelude::*;
use sweet::*;

#[sweet_test(non_send)]
pub async fn works() -> Result<()> {
	// expect(true).to_be_false()?;

	let topic = Topic::pubsub_update("foo/bar");

	let relay = Relay::default();

	let _sub1 = relay.add_subscriber_with_topic::<u8>(&topic)?;


	Ok(())
}
