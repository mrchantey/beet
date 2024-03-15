use super::test_constant_behavior_tree;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();
	let target = app.world.spawn_empty().id();
	let entities = test_constant_behavior_tree().spawn(&mut app, target);

	let entity = *entities.node(2).unwrap();
	app.world.entity_mut(entity).insert(Score::Pass);
	let scores = ComponentGraph::<Score>::from_world(&app.world, &entities);

	expect(scores.node(0)).to_be(Some(&Some(&Score::Fail)))?;
	expect(scores.node(1)).to_be(Some(&Some(&Score::Fail)))?;
	expect(scores.node(2)).to_be(Some(&Some(&Score::Pass)))?;
	expect(scores.node(3)).to_be(Some(&Some(&Score::Fail)))?;

	Ok(())
}
