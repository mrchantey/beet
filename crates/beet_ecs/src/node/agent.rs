#[allow(unused)]
use crate::prelude::*;
use bevy::prelude::*;

/// Added to entites that have at least one associated behavior graph.
/// Remove this component to dispose of all of this agents graphs.
/// This is useful, for example for [`cleanup_entity_graph`] to only listen for removals
/// of agent entities
#[derive(Debug, Copy, Clone, PartialEq, Component)]
pub struct AgentMarker;

/// Added to [`BehaviorNode`] entities that have a target agent.
#[derive(Debug, PartialEq, Deref, DerefMut, Component)]
pub struct TargetAgent(pub Entity);


/// Used by actions to specify some target, ie seek.
#[derive(Debug, PartialEq, Deref, DerefMut, Component)]
pub struct ActionTarget(pub Entity);

/// Removes all nodes with a [`TargetAgent`] component that matches the removed agent
pub fn despawn_graph_on_agent_removed(
	mut commands: Commands,
	nodes: Query<(Entity, &TargetAgent)>,
	mut removed_agents: RemovedComponents<AgentMarker>,
) {
	for agent in removed_agents.read() {
		for (node, target) in nodes.iter() {
			if **target == agent {
				commands.entity(node).despawn();
			}
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn despawn() -> Result<()> {
		let mut app = App::new();
		// app.add_systems(PostUpdate, despawn_graph_on_agent_removed);
		app.add_plugins(BeetSystemsPlugin::<EcsNode, _>::default());

		let target = app.world.spawn_empty().id();
		InsertOnRun(RunResult::Success)
			.into_beet_builder()
			.spawn(&mut app.world, target);

		expect(app.world.entities().len()).to_be(2)?;
		app.update();
		app.world.despawn(target);

		expect(app.world.entities().len()).to_be(1)?;
		app.update();
		expect(app.world.entities().len()).to_be(0)?;

		Ok(())
	}
}
