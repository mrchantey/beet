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
	let tx = relay_tx.add_publisher_with_topic::<u32>(topic.clone())?;

	let relay_rx = Relay::default();
	let _pm_rx = PostMessageRelay::new_with_current_window(relay_rx.clone());
	let mut rx = relay_rx.add_subscriber_with_topic::<u32>(topic)?;

	tx.push(&8)?;

	pm_tx.send_all()?;

	wait_for_millis(1).await;

	expect(rx.try_recv()?).to_be(8)?;

	Ok(())
}
