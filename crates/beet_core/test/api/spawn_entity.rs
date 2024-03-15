use beet_core::prelude::*;
use beet_ecs::prelude::*;
use beet_net::prelude::*;
use bevy::prelude::*;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();
	let mut relay = Relay::default();
	app.add_plugins(BeetPlugin::<EcsNode>::new(relay.clone()));

	SpawnEntityHandler::<EcsNode>::publisher(&mut relay)?.push(
		&SpawnEntityPayload::from_id(0).with_position(Vec3::new(0., 0., 0.)),
	)?;

	expect(&app.world.iter_entities().count()).to_be(&0)?;
	app.update();
	expect(&app.world.iter_entities().count()).to_be(&1)?;

	Ok(())
}

#[sweet_test]
pub fn pubsub() -> Result<()> {
	let mut relay = Relay::default();

	let mut subscriber =
		SpawnEntityHandler::<CoreNode>::subscriber(&mut relay)?;
	let beet_id = BeetEntityId(0);

	SpawnEntityHandler::<CoreNode>::publisher(&mut relay)?.push(
		&SpawnEntityPayload::from_id(beet_id)
			.with_position(Vec3::new(0., 0., 0.))
			// .with_prefab(EmptyAction.into_prefab()?),
			.with_prefab(
				Translate::new(Vec3::new(1., 0., 0.)).into_prefab()?.typed(),
			),
	)?;

	let _result = subscriber.try_recv()?;
	// expect(result.beet_id).to_be(beet_id)?;

	Ok(())
}
