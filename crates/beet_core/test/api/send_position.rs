use beet_core::prelude::*;
use bevy_app::prelude::*;
use bevy_math::Vec3;
use sweet::*;

#[sweet_test(non_send)]
pub async fn works() -> Result<()> {
	let mut app = App::new();
	let mut relay = Relay::default();
	app.add_plugins(BeetPlugin::<EcsNode>::new(relay.clone()));

	let beet_id = BeetEntityId(0);

	SpawnEntityHandler::<EcsNode>::publisher(&mut relay)?.push(
		&SpawnEntityPayload::from_id(beet_id)
			.with_tracked_position(Vec3::new(0., 1., 0.)),
	)?;

	app.update();

	let topic = PositionSender::topic(beet_id);
	let recv: Vec3 = relay.recv(topic)?;
	expect(recv).to_be(Vec3::new(0., 1., 0.))?;

	Ok(())
}
