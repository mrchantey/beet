use beet_core::prelude::*;
use beet_ecs::graph::IntoBehaviorPrefab;
use bevy_app::prelude::*;
use bevy_math::Vec3;
use std::sync::atomic::AtomicUsize;
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

static GOOD: AtomicUsize = AtomicUsize::new(0);
const BAD: AtomicUsize = AtomicUsize::new(0);
#[sweet_test]
fn beet_entity_id() -> Result<()> {
	expect(GOOD.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as u64)
		.to_be(0)?;
	expect(GOOD.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as u64)
		.to_be(1)?;
	expect(BAD.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as u64)
		.to_be(0)?;
	expect(BAD.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as u64)
		.to_be(0)?;
	expect(BeetEntityId::next()).to_be(0.into())?;
	expect(BeetEntityId::next()).to_be(1.into())?;
	expect(BeetEntityId::next()).to_be(2.into())?;
	expect(BeetEntityId::next()).to_be(3.into())?;

	Ok(())
}

#[sweet_test]
fn serde_bytes() -> Result<()> {
	let prefab1 = BehaviorPrefab::<EcsNode>::from_graph(ConstantScore(
		Score::Weight(0.5),
	))?;
	let bytes1 = bincode::serialize(&prefab1)?;
	let prefab2: BehaviorPrefab<EcsNode> = bincode::deserialize(&bytes1)?;
	let bytes2 = bincode::serialize(&prefab2)?;
	expect(bytes1).to_be(bytes2)?;
	Ok(())
}
