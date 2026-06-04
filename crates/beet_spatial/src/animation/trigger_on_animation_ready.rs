use beet_action::prelude::*;
use beet_core::prelude::*;

/// Attach as a descendent of a parent with a [`WorldAssetRoot`] and the
/// entity's action will be called with the stored payload once the
/// [`AnimationPlayer`] is ready.
///
/// The default [`trigger_on_animation_ready::<()>`] system is added by
/// [`AnimationFlowPlugin`]; other payload types must be added manually.
///
/// ## How it works
/// The [`AnimationPlayer`] api is a bit awkward to work with:
/// 1. Listen for an [`AnimationPlayer`] to be added.
/// 2. Find the parent [`WorldAssetRoot`].
/// 3. Find all descendants with a [`TriggerOnAnimationReady`] component.
/// 4. Call the action on those entities with the stored payload.
#[derive(Component)]
pub struct TriggerOnAnimationReady<P = ()>
where
	P: 'static + Send + Sync + Clone,
{
	/// The input passed to the action when the animation player is ready.
	pub payload: P,
}

impl<P> TriggerOnAnimationReady<P>
where
	P: 'static + Send + Sync + Clone,
{
	/// Create a new [`TriggerOnAnimationReady`] with `payload`.
	pub fn new(payload: P) -> Self { Self { payload } }
}

impl TriggerOnAnimationReady<()> {
	/// Create a [`TriggerOnAnimationReady`] that calls its action with `()`.
	pub fn run() -> Self { Self { payload: () } }
}

/// Calls the action of any [`TriggerOnAnimationReady<P>`] entity under a
/// [`WorldAssetRoot`] once its [`AnimationPlayer`] has loaded, passing a clone
/// of the stored payload as input.
pub fn trigger_on_animation_ready<P>(
	mut commands: Commands,
	scene_roots: Query<Entity, With<WorldAssetRoot>>,
	parents: Query<&ChildOf>,
	children: Query<&Children>,
	actions: Query<(Entity, &TriggerOnAnimationReady<P>)>,
	players: Populated<Entity, Added<AnimationPlayer>>,
) where
	P: 'static + Send + Sync + Clone,
{
	for entity in players.iter() {
		for root in parents
			.iter_ancestors_inclusive(entity)
			.filter_map(|entity| scene_roots.get(entity).ok())
		{
			for child in children.iter_descendants_inclusive(root) {
				if let Ok((action, trigger)) = actions.get(child) {
					commands.entity(action).call::<P, Outcome>(
						trigger.payload.clone(),
						OutHandler::default(),
					);
				}
			}
		}
	}
}
