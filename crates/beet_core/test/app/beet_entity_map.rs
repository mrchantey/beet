use beet_core::prelude::*;
use beet_ecs::graph::IntoBehaviorPrefab;
use bevy_app::prelude::*;
use bevy_math::Vec3;
use sweet::*;



#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();
	let mut relay = Relay::default();

	app.insert_time()
		.add_plugins(BeetPlugin::<CoreNode>::new(relay.clone()));

	let beet_id = BeetEntityId(0);

	expect(app.world.entities().len()).to_be(0)?;
	SpawnEntityHandler::<CoreNode>::publisher(&mut relay)?.push(
		&SpawnEntityPayload::from_id(beet_id)
			.with_tracked_position(Vec3::ZERO)
			.with_prefab(Translate::new(Vec3::new(0., 1., 0.)).into_prefab()?),
	)?;

	app.update();

	expect(app.world.entities().len()).to_be(2)?;

	DespawnEntityHandler::publisher(&mut relay)?
		.push(&DespawnEntityPayload::new(beet_id))?;

	app.update();

	expect(app.world.entities().len()).to_be(0)?;

	expect(app.world.resource::<BeetEntityMap>().map().len()).to_be(0)?;
	Ok(())
}
