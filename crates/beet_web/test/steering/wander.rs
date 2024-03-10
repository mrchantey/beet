use beet::prelude::*;
use beet_web::prelude::*;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	append_html_for_tests();
	let mut relay = Relay::default();

	BeeGame::create_bee_pub(&mut relay)
		.push(&BehaviorTree::new(Wander).into())?;

	run(relay);
	Ok(())
}
