use beet_net::topic::TopicAddress;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let address = "foo:0/bar";

	let _addr = TopicAddress::new(address);

	// let topic = Topic::pubsub_update(address);





	// expect(true).to_be_false()?;

	Ok(())
}
