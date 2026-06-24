//! Contains the animation systems and components.
//! See [`AnimationFlowPlugin`] for more information.
mod init_animators;
mod trigger_on_animation_ready;
use self::init_animators::*;
mod trigger_on_animation_end;
pub use self::trigger_on_animation_end::*;
mod play_animation;
pub use self::play_animation::*;
mod resolve_animation_clips;
use self::resolve_animation_clips::*;
pub use self::trigger_on_animation_ready::*;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// A plugin containing systems required for animation actions:
/// - [`PlayAnimation`]
/// - [`TriggerOnAnimationEnd`]
/// Note that this plugin also adds an opinionated [`init_animators`] system
/// that will append the [`AnimationTransitions`] ( and [`AnimationGraphHandle`] if found in parents)
/// components to all entities with an [`AnimationPlayer`] component.
#[derive(Default)]
pub struct AnimationFlowPlugin;

impl Plugin for AnimationFlowPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<PlayAnimation>()
			.register_type::<TriggerOnAnimationReady<()>>()
			.register_type::<TriggerOnAnimationEnd<Outcome>>()
			.add_systems(
				Update,
				(
					init_animators,
					resolve_animation_clips,
					trigger_on_animation_ready::<()>,
				)
					.chain(),
			)
			.add_systems(
				Update,
				trigger_on_animation_end::<Outcome>.in_set(TickSet),
			);
	}
}
