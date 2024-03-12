use beet_core::prelude::*;
use bevy_app::App;
use bevy_math::Vec3;
use bevy_transform::components::Transform;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();
	let mut relay = Relay::default();
	app.add_plugins(BeetPlugin::<CoreNode>::new(relay.clone()));
	app.insert_time();

	let beet_id = BeetEntityId(0);

	SpawnEntityHandler::<CoreNode>::publisher(&mut relay)?.push(
		&SpawnEntityPayload::from_id(beet_id)
			.with_prefab(Translate::new(Vec3::new(1., 0., 0.)).into_prefab()?)
			.with_position(Vec3::new(-1., 0., 0.)),
	)?;

	expect(&app.world.iter_entities().count()).to_be(&0)?;
	app.update_with_secs(2);
	expect(app.world.entities().len()).to_be(2)?;


	let entity = app
		.world
		.resource::<BeetEntityMap>()
		.map()
		.get(&beet_id)
		.unwrap();

	let translation = app
		.world
		.entity(*entity)
		.get::<Transform>()
		.unwrap()
		.translation;
	expect(translation).to_be(Vec3::new(1., 0., 0.))?;

	Ok(())
}
