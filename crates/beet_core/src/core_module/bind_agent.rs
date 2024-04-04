use beet::graph::BeetPrefab;
use beet::graph::BeetRoot;
use beet::tree::BeetNode;
use bevy::prelude::*;



#[derive(Copy, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct BindAgentToFirstGraph;




pub fn bind_agent_to_first_graph(
	world: &mut World, // mut commands: Commands,
	                   // roots: Query<Entity, >,
	                   // query: Query<Entity, With<BindAgentToFirstGraph>>,
) {
	for entity in world
		.query_filtered::<Entity, With<BindAgentToFirstGraph>>()
		.iter(world)
		.collect::<Vec<_>>()
		.into_iter()
	{
		let Some(first) = world
			.query_filtered::<Entity, (With<BeetRoot>, With<BeetPrefab>)>()
			.iter(world)
			.next()
		else {
			log::warn!("No first graph found to bind agent to");
			continue;
		};

		let new_node = BeetNode(first).deep_clone(world).unwrap();
		new_node.bind_agent(world, entity);

		world.entity_mut(entity).remove::<BindAgentToFirstGraph>();
	}
}
