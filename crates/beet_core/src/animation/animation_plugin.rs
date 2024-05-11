use super::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
	#[rustfmt::skip]
	fn build(&self, app: &mut App) {
		app
    	.add_systems(Update, init_animators)
			.add_plugins(ActionPlugin::<(
				PlayAnimation,
				InsertOnAnimationEnd<RunResult>
			)>::default())
		;
	}
}
