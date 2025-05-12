//! Contains the animation systems and components.
//! See [`AnimationPlugin`] for more information.
mod init_animators;
mod run_on_animation_ready;
use self::init_animators::*;
mod return_on_animation_end;
pub use self::return_on_animation_end::*;
mod play_animation;
pub use self::play_animation::*;
pub use self::run_on_animation_ready::*;
use beet_flow::prelude::*;
use bevy::prelude::*;


/// A plugin containing systems required for animation actions:
/// - [`PlayAnimation`]
/// - [`TriggerOnAnimationEnd`]
/// Note that this plugin also adds an opinionated [`init_animators`] system
/// that will append the [`AnimationTransitions`] ( and [`AnimationGraphHandle`] if found in parents)
/// components to all entities with an [`AnimationPlayer`] component.
#[derive(Default)]
pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			(init_animators, run_on_animation_ready::<()>).chain(),
		)
		.add_systems(
			Update,
			(
				// play_animation_on_load,
				return_on_animation_end::<RunResult>,
			)
				.in_set(TickSet),
		);
		// .never_param_warn()
	}
}
