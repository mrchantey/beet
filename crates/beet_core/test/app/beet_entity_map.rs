use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy_app::prelude::*;
use bevy_math::Vec3;
use sweet::*;



#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();
	let mut relay = Relay::default();

	app.insert_time()
		.add_plugins(BeetPlugin::<CoreNode>::new(relay.clone()));

	let mut create = SpawnEntityHandler::<CoreNode>::requester(&mut relay);

	expect(app.world.entities().len()).to_be(0)?;
	let message_id = create.start_request(
		&SpawnEntityPayload::default()
			.with_tracked_position(Vec3::ZERO)
			.with_graph(translate_graph()),
	)?;

	app.update();

	let beet_id = create.block_on_response(message_id)?;

	expect(app.world.entities().len()).to_be(2)?;

	let mut delete = DespawnEntityHandler::requester(&mut relay);

	let message_id =
		delete.start_request(&DespawnEntityPayload::new(beet_id))?;

	app.update();

	expect(delete.block_on_response(message_id)).to_be_ok()?;

	expect(app.world.entities().len()).to_be(0)?;

	expect(app.world.resource::<BeetEntityMap>().map().len()).to_be(0)?;
	Ok(())
}
