use beet_net::prelude::*;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let topic = Topic::pubsub_update("a");

	let msg = StateMessage::new(topic, &9u8, 0)?;

	expect(msg.payload::<u16>())
		.to_be_err_str("Type mismatch for a\nexpected u8, received u16")?;
	expect(msg.payload::<u8>()?).to_be(9)?;


	Ok(())
}
