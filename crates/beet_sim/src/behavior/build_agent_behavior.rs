use crate::prelude::*;
use anyhow::Result;
use beet_flow::prelude::*;
use bevy::prelude::*;

/// Build behavior for an agent, depending on its
/// abilities and desires.
pub fn build_agent_behavior(
	world: &mut World,
	agent: Entity,
) -> Result<Entity> {
	let Some(walk) = world.get::<Walk>(agent) else {
		anyhow::bail!("currenty only agents with walk ability are supported");
	};
	let _walk = walk.clone();

	let behavior = world
		.spawn((
			Name::new("Behavior"),
			Emoji::new("1F600"),
			TargetEntity(agent),
			ScoreFlow::default(),
			Repeat::default(),
		))
		.id();


	world.entity_mut(agent).add_child(behavior);


	Ok(behavior)
}
