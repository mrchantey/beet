use beet_core::prelude::*;
use beet_ecs::ecs_nodes::EcsNode;
use beet_net::relay::Relay;
use bevy_app::App;
use bevy_math::Vec3;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();
	let mut relay = Relay::default();
	app.add_plugins(BeetPlugin::<EcsNode>::new(relay.clone()));

	let mut send = SpawnEntityHandler::<EcsNode>::requester(&mut relay);
	let message_id = send.start_request(
		&SpawnEntityPayload::default().with_position(Vec3::new(0., 0., 0.)),
	)?;

	app.update();

	let id = send.block_on_response(message_id)?;
	expect(*id).to_be(0)?;

	Ok(())
}
