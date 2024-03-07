use beet_core::prelude::*;
use beet_ecs::prelude::*;
use beet_net::prelude::*;
use bevy_app::prelude::*;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();
	let relay = Relay::default();

	app.add_plugins(BeetPlugin::<EcsNode>::new(relay.clone()));


	expect(app.world.entities().len()).to_be(0)?;



	Ok(())
}
