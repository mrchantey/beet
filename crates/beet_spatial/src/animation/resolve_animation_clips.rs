use super::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use bevy::animation::graph::AnimationNodeType;

/// When an agent's [`AnimationPlayer`] loads, resolve the clip-PATH on its
/// behaviour-tree [`PlayAnimation`] / [`TriggerOnAnimationEnd`] actions to the
/// graph node index, by matching the loaded clip handle against the agent's
/// [`AnimationGraph`] nodes.
///
/// The agent is the [`WorldAssetRoot`] ancestor (the spawned-model root, eg the
/// `<Foxie>` entity) that carries the [`AnimationGraphHandle`], not the player
/// itself: `init_animators` copies the handle onto the player entity, but that is
/// inside the glb subtree while the behaviour-tree actions hang off the model root.
pub(super) fn resolve_animation_clips(
	asset_server: When<Res<AssetServer>>,
	graphs: When<Res<Assets<AnimationGraph>>>,
	parents: Query<&ChildOf>,
	children: Query<&Children>,
	agents: Query<&AnimationGraphHandle, With<WorldAssetRoot>>,
	players: Populated<Entity, Added<AnimationPlayer>>,
	mut play: Query<&mut PlayAnimation>,
	mut end: Query<&mut TriggerOnAnimationEnd<Outcome>>,
) -> Result {
	for player in players.iter() {
		let Some((agent, handle)) =
			parents.iter_ancestors_inclusive(player).find_map(|entity| {
				agents.get(entity).ok().map(|handle| (entity, handle))
			})
		else {
			continue;
		};
		let Some(graph) = graphs.get(&handle.0) else {
			continue;
		};
		// match an owned clip path against the graph's loaded clip nodes
		let index_of = |path: &str| -> Option<AnimationNodeIndex> {
			let wanted = asset_server.load::<AnimationClip>(path.to_string());
			graph.nodes().find(|node| {
				matches!(
					graph.get(*node).map(|node| &node.node_type),
					Some(AnimationNodeType::Clip(clip)) if *clip == wanted
				)
			})
		};
		for entity in children.iter_descendants_inclusive(agent) {
			if let Ok(mut action) = play.get_mut(entity) {
				if let Some(index) = index_of(&action.clip) {
					action.animation = index;
				}
			}
			if let Ok(mut action) = end.get_mut(entity) {
				if let Some(index) = index_of(&action.clip) {
					action.handle = asset_server.load(action.clip.to_string());
					action.animation_index = index;
				}
			}
		}
	}
	Ok(())
}
