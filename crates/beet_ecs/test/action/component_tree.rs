use super::test_constant_behavior_tree;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let mut world = World::new();
	let target = world.spawn_empty().id();
	let tree = test_constant_behavior_tree().spawn(&mut world, target);
	expect(tree.children.len()).to_be(2)?;

	let entity = tree.children[1].value;
	world.entity_mut(entity).insert(Score::Pass);
	let scores = tree.component_tree::<Score>(&world);

	expect(scores.value).to_be(Some(&Score::Fail))?;
	expect(scores.children[0].value).to_be(Some(&Score::Fail))?;
	expect(scores.children[1].value).to_be(Some(&Score::Pass))?;
	expect(scores.children[1].children[0].value).to_be(Some(&Score::Fail))?;

	Ok(())
}
