use beet_core::prelude::*;
use beet_flow::prelude::*;

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
/// 3. Find all children with a [`TriggerOnAnimationReady`] component.
/// 4. Trigger the action on those children with the [`SceneRoot`] as
/// 	the [`OnRun::origin`].
///
#[derive(Component)]
pub struct TriggerOnAnimationReady<P> {
	/// The action to trigger.
	pub payload: P,
}

impl TriggerOnAnimationReady<RequestEndResult> {
	/// Create a new [`TriggerOnAnimationReady`] with a `RequestEndResult` payload.
	pub fn run() -> Self {
		Self {
			payload: Default::default(),
		}
	}
}

/// The associated system for [`TriggerOnAnimationReady`].
/// The defaullt [`OnRun<()>`] is added in the [`AnimationPlugin`],
/// any other payload types must be added manually.
pub fn trigger_on_animation_ready<P: IntoEntityEvent + Clone>(
	mut commands: Commands,
	scene_roots: Query<Entity, With<SceneRoot>>,
	parents: Query<&ChildOf>,
	children: Query<&Children>,
	actions: Query<(Entity, &TriggerOnAnimationReady<P>)>,
	players: Populated<Entity, Added<AnimationPlayer>>,
) {
	for entity in players.iter() {
		for root in parents
			.iter_ancestors_inclusive(entity)
			.filter_map(|entity| scene_roots.get(entity).ok())
		{
			for child in children.iter_descendants_inclusive(root) {
				if let Ok((action, trigger)) = actions.get(child) {
					commands
						.entity(action)
						.trigger_entity(trigger.payload.clone());
				}
			}
		}
	}
}
