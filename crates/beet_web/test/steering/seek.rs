use beet::prelude::*;
use beet_web::prelude::*;
use bevy_math::Vec3;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	append_html_for_tests();
	let mut relay = Relay::default();

	BeeGame::create_bee_pub(&mut relay)
		.push(&BehaviorTree::new(Seek).into())?;
	BeeGame::create_flower_pub(&mut relay).push(&Vec3::new(-0.5, 0., 0.))?;

	run(relay);
	Ok(())
}
