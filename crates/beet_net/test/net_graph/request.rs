use beet_net::prelude::*;
use sweet::*;


#[sweet_test(non_send)]
pub async fn request() -> Result<()> {
	pretty_env_logger::try_init().ok();
	let relay = Relay::default();
	let topic = TopicAddress::new("foo/bar");
	let mut req = relay
		.add_requester::<u8, u8>(&topic, TopicMethod::Update)
		.await?;
	let mut res = relay
		.add_responder::<u8, u8>(&topic, TopicMethod::Update)
		.await?;

	// let mock_fn = mock_func(|val| val);
	// let mock_fn2 = mock_fn.clone();

	let handle = tokio::spawn(async move {
		// let mock_fn = mock_fn2.clone();
		res.handle_requests_forever(|req| {
			let val = req * 2;
			// log::info!("wadduip");
			// mock_fn.clone().call(val);
			val
		})
		.await
	});
	// expect(&mock_fn).to_have_returned_with(&64)?;

	let res = req.request(&32).await?;
	expect(res).to_be(64)?;
	handle.abort();
	Ok(())
}
