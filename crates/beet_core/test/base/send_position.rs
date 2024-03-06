use beet_core::prelude::*;
use beet_ecs::builtin_nodes::BuiltinNode;
use beet_net::prelude::*;
use bevy_app::prelude::*;
use bevy_math::Vec3;
use sweet::*;

#[sweet_test(non_send)]
pub async fn works() -> Result<()> {
	let mut app = App::new();
	let mut relay = Relay::default();
	app.add_plugins(BeetPlugin::<BuiltinNode>::new(relay.clone()));


	let mut send = SpawnEntityHandler::requester(&mut relay);
	let message_id = send.start_request(
		&SpawnEntityPayload::default()
			.with_position(Vec3::new(0., 1., 0.))
			.with_position_tracking(),
	)?;

	app.update();

	let id = send.block_on_response(message_id)?;
	expect(*id).to_be(0)?;
	let topic = PositionSender::topic(id);
	let recv: Vec3 = relay.recv(topic)?;
	expect(recv).to_be(Vec3::new(0., 1., 0.))?;

	Ok(())
}
