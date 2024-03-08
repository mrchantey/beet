use beet_net::prelude::*;
use sweet::*;



#[sweet_test(non_send, skip)]
async fn receives_topic_graph_changed() -> Result<()> {
	let mut relay1 = Relay::default();

	let topic = Topic::pubsub_update("foo/bar");

	let _pub1 = relay1.add_publisher_with_topic::<u32>(topic)?;
	let messages = relay1.get_all_messages()?;

	expect(messages.len()).to_be(1)?;

	Ok(())
}
#[sweet_test(non_send)]
async fn cross_boundary_topic_changed() -> Result<()> {
	let topic = Topic::pubsub_update("foo/bar");
	let mut relay1 = Relay::default();
	let mut relay2 = Relay::default();

	let pub1 = relay1.add_publisher_with_topic::<u32>(&topic)?;
	let mut sub2 = relay2.add_subscriber_with_topic::<u32>(&topic)?;

	pub1.push(&8)?;

	relay1.sync_local(&mut relay2).await?;
	expect(sub2.try_recv()?).to_be(8)?;

	Ok(())
}

#[sweet_test(non_send)]
async fn cross_boundary_errors() -> Result<()> {
	let topic = Topic::pubsub_update("foo/bar");

	let mut relay1 = Relay::default();
	let pub1 = relay1.add_publisher_with_topic::<u32>(&topic)?;

	let mut relay2 = Relay::default();

	pub1.push(&8)?;
	let mut sub2 = relay2.add_subscriber_with_topic::<u8>(&topic)?;
	relay1.sync_local(&mut relay2).await?;
	expect(sub2.try_recv()).to_be_err_str(
		"Type mismatch for foo/bar\nexpected u32, received u8",
	)?;



	Ok(())
}
