use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;

#[derive(Default)]
pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
	#[rustfmt::skip]
	fn build(&self, app: &mut App) {
		app
    	.add_systems(Update, init_animators)
			.add_plugins(ActionPlugin::<(
				PlayAnimation,
				PlayProceduralAnimation,
				TriggerOnAnimationEnd<OnRunResult>
			)>::default())
		;
	}
}
