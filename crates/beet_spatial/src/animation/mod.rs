mod animation_action_bundle;
pub use self::animation_action_bundle::*;
mod init_animators;
pub use self::init_animators::*;
mod insert_on_animation_end;
pub use self::insert_on_animation_end::*;
mod play_animation;
pub use self::play_animation::*;
use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;

#[derive(Default)]
pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, init_animators)
			.add_plugins(ActionPlugin::<(
				PlayAnimation,
				PlayProceduralAnimation,
				SetCurveOnRun,
				TriggerOnAnimationEnd<OnResult>,
			)>::default())
			.add_systems(
				Update,
				(play_animation_on_load, trigger_on_animation_end::<OnResult>)
					.in_set(TickSet),
			)
			.never_param_warn()
			.in_set(TickSet);
	}
}
