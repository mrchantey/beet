use beet::prelude::*;
use beet_web::prelude::*;
use bevy_math::prelude::*;
use sweet::*;

#[sweet_test]
pub async fn works() -> Result<()> {
	append_html_for_tests();

	let mut relay = Relay::default();
	BeeGame::create_bee_pub(&mut relay).push(
		&BehaviorTree::new(Translate::new(Vec3::new(-0.1, 0., 0.))).into(),
	)?;
	BeeGame::create_flower_pub(&mut relay).push(&())?;

	run(relay);
	Ok(())
}
