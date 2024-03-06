use beet_net::prelude::*;
use sweet::*;


#[sweet_test(non_send)]
pub async fn request() -> Result<()> {
	pretty_env_logger::try_init().ok();
	let relay = Relay::default();
	let topic = TopicAddress::new("foo/bar");
	let mut req = relay.add_requester::<u8, u8>(&topic, TopicMethod::Update)?;
	let mut res = relay.add_responder::<u8, u8>(&topic, TopicMethod::Update)?;

	let id = req.start_request(&32)?;
	res.try_handle_next(|val| val * 2)?;

	let res = req.block_on_response(id)?;
	expect(res).to_be(64)?;
	Ok(())
}
