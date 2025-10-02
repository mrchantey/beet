use beet_core::prelude::*;

/// Once an [`AnimationPlayer`] is loaded,
/// add the additional components needed to play animations.
/// This is required by actions like [`PlayAnimation`]
/// that need [`AnimationTransitions`] to trigger animations.
pub(crate) fn init_animators(
	mut commands: Commands,
	parents: Query<&ChildOf>,
	graphs: Query<&AnimationGraphHandle>,
	players: Populated<Entity, Added<AnimationPlayer>>,
) {
	for entity in players.iter() {
		if let Some(graph) = parents
			.iter_ancestors_inclusive(entity)
			.find_map(|entity| graphs.get(entity).ok())
		{
			commands.entity(entity).insert(graph.clone());
		}
		commands.entity(entity).insert(AnimationTransitions::new());
	}
}
