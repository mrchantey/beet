//! Contains the animation systems and components.
//! See [`AnimationFlowPlugin`] for more information.
mod init_animators;
mod trigger_on_animation_ready;
use self::init_animators::*;
mod trigger_on_animation_end;
pub use self::trigger_on_animation_end::*;
mod play_animation;
pub use self::play_animation::*;
pub use self::trigger_on_animation_ready::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;


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
		app.add_systems(
			Update,
			(
				init_animators,
				trigger_on_animation_ready::<RequestEndResult>,
			)
				.chain(),
		)
		.add_systems(
			Update,
			(
				// play_animation_on_load,
				trigger_on_animation_end::<EndResult>,
			)
				.in_set(TickSet),
		);
		// .never_param_warn()
	}
}
