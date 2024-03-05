use beet::prelude::*;
use beet_web::prelude::*;
use forky_web::wait_for_millis;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	console_log::init().ok();

	let topic = Topic::pubsub_update("foo/bar");

	let relay_tx = Relay::default();
	let mut pm_tx = PostMessageRelay::new_with_current_window(relay_tx.clone());
	let tx = relay_tx.add_publisher::<u32>(topic.clone()).await?;

	let relay_rx = Relay::default();
	let _pm_rx = PostMessageRelay::new_with_current_window(relay_rx.clone());
	let mut rx = relay_rx.add_subscriber::<u32>(topic).await?;

	tx.broadcast(&8).await?;

	pm_tx.send_all()?;

	wait_for_millis(1).await;

	let msg = rx.recv_default_timeout().await?;

	expect(msg).to_be(8)?;

	Ok(())
}
