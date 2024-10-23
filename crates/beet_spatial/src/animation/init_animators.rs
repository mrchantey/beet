use beet_flow::extensions::ParentExt;
use bevy::prelude::*;


/// Once an [`AnimationPlayer`] is loaded, add the additional components needed to play animations.
pub fn init_animators(
	mut commands: Commands,
	parents: Query<&Parent>,
	graphs: Query<&AnimationGraphHandle>,
	mut players: Query<Entity, Added<AnimationPlayer>>,
) {
	for entity in &mut players {
		if let Some(graph) =
			ParentExt::find(entity, &parents, |entity| graphs.get(entity).ok())
		{
			commands.entity(entity).insert(graph.clone());
		}
		commands.entity(entity).insert(AnimationTransitions::new());
	}
}
