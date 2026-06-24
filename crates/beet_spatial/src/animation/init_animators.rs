use beet_core::prelude::*;

/// Copies the [`AnimationGraphHandle`] (and adds [`AnimationTransitions`]) from
/// the model root onto each newly loaded [`AnimationPlayer`].
///
/// Load-bearing: bevy's `advance_animations`/`animate_targets` silently skip any
/// [`AnimationPlayer`] lacking an [`AnimationGraphHandle`], but the glb spawns the
/// player bare, so the handle has to be copied down from the model root. Scene
/// spawn gating guarantees the player exists, not that it carries the handle, so
/// this stays. [`AnimationTransitions`] is likewise required by actions such as
/// [`PlayAnimation`] to trigger animations.
///
/// Filtered on `Without<AnimationTransitions>` rather than `Added<AnimationPlayer>`:
/// the spawned player is initialised on the first frame it lacks the transitions,
/// so a `RunOnLoad`-started tree never out-races a one-shot `Added` setup. The
/// query is empty (and the system a no-op) once every player is initialised.
pub(crate) fn init_animators(
	mut commands: Commands,
	parents: Query<&ChildOf>,
	graphs: Query<&AnimationGraphHandle>,
	players: Populated<
		Entity,
		(With<AnimationPlayer>, Without<AnimationTransitions>),
	>,
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
