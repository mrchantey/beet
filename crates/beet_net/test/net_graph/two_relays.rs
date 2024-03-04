use beet_net::prelude::*;
use sweet::*;



#[sweet_test(non_send)]

async fn receives_topic_graph_changed() -> Result<()> {
	let mut relay1 = Relay::default();

	let topic = Topic::pubsub_update("foo/bar");

	let _pub1 = relay1.add_publisher::<u32>(topic).await?;
	let messages = relay1.get_all_messages()?;

	expect(messages.len()).to_be(1)?;

	Ok(())
}
#[sweet_test(non_send)]

async fn cross_boundary_topic_changed() -> Result<()> {
	let topic = Topic::pubsub_update("foo/bar");
	let mut relay1 = Relay::default();
	let mut relay2 = Relay::default();

	let pub1 = relay1.add_publisher::<u32>(&topic).await?;
	let mut sub2 = relay2.add_subscriber::<u32>(&topic).await?;

	pub1.broadcast(&8).await?;

	relay1.sync_local(&mut relay2).await?;
	expect(sub2.recv_default_timeout().await?).to_be(8)?;

	Ok(())
}

#[sweet_test(non_send)]

async fn cross_boundary_errors() -> Result<()> {
	let topic = Topic::pubsub_update("foo/bar");

	let mut relay1 = Relay::default();
	let pub1 = relay1.add_publisher::<u32>(&topic).await?;

	let mut relay2 = Relay::default();

	pub1.broadcast(&8).await?;
	let mut sub2 = relay2.add_subscriber::<u8>(&topic).await?;
	relay1.sync_local(&mut relay2).await?;
	expect(sub2.recv_default_timeout().await).to_be_err_str(
		"Type mismatch for foo/bar:0\nexpected u32, received u8",
	)?;



	Ok(())
}
