use anyhow::Result;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use sweet::*;


pub fn expect_tree<T>(
	app: &mut App,
	entity_graph: &EntityGraph,
	expected: Tree<Option<&T>>,
) -> Result<()>
where
	T: Component + PartialEq + std::fmt::Debug,
{
	let running_tree =
		ComponentGraph::<T>::from_world(&app.world, &entity_graph)
			.clone()
			.into_tree();
	expect(running_tree).to_be(expected)
}
