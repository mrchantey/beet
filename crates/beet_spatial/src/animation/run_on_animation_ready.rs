use beet_flow::prelude::*;
use bevy::prelude::*;
use sweet::prelude::HierarchyQueryExtExt;

///
/// Attach as a descendent of a parent with a [`SceneRoot`] and it will
/// run when the [`AnimationPlayer`] is ready.
///
/// The system is added for [`OnRun<()>`] in the [`AnimationPlugin`],
/// any other payload types must be added manually.
///
/// ## How it works
/// The [`AnimationPlayer`] api is a bit awkward to work with:
/// 1. Listen for an [`AnimationPlayer`] to be added.
/// 2. Find the parent [`SceneRoot`].
/// 3. Find all children with a [`RunOnAnimationReady`] component.
/// 4. Trigger the action on those children with the [`SceneRoot`] as
/// 	the [`OnRun::origin`].
///
#[derive(Component)]
pub struct RunOnAnimationReady<P> {
	/// The action to trigger.
	pub payload: P,
}

impl Default for RunOnAnimationReady<()> {
	fn default() -> Self {
		Self {
			payload: Default::default(),
		}
	}
}

/// The associated system for [`RunOnAnimationReady`].
/// The defaullt [`OnRun<()>`] is added in the [`AnimationPlugin`],
/// any other payload types must be added manually.
pub fn run_on_animation_ready<P: RunPayload>(
	mut commands: Commands,
	scene_roots: Query<Entity, With<SceneRoot>>,
	parents: Query<&Parent>,
	children: Query<&Children>,
	actions: Query<&RunOnAnimationReady<P>>,
	players: Populated<Entity, Added<AnimationPlayer>>,
) {
	for entity in players.iter() {
		for parent in parents.iter_ancestors_inclusive(entity) {
			if let Ok(scene_root) = scene_roots.get(parent) {
				for child in children.iter_descendants_inclusive(parent) {
					if let Ok(action) = actions.get(child) {
						commands.trigger(OnRunAction::new(
							child,
							scene_root,
							action.payload.clone(),
						));
					}
				}
			}
		}
	}
}
