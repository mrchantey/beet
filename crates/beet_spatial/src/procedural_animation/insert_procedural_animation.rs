use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;


/// Inserts a [`PlayProceduralAnimation`] whenever an [`OnRun`] trigger is received
#[derive(Debug, Clone, PartialEq, Component, Reflect, Action)]
#[observers(insert_procedural_animation)]
#[reflect(Default, Component)]
#[require(PlayProceduralAnimation)]
pub struct InsertProceduralAnimation {
	/// The `end` of the last animation
	/// to be used as the `start` of the next animation
	pub last_dir: Dir2,
	pub func: EaseFunction,
}

impl Default for InsertProceduralAnimation {
	fn default() -> Self {
		Self {
			func: EaseFunction::CubicInOut,
			last_dir: Dir2::X,
		}
	}
}

fn insert_procedural_animation(
	trigger: Trigger<OnRun>,
	mut query: Query<(
		&mut InsertProceduralAnimation,
		&mut PlayProceduralAnimation,
	)>,
) {
	let (mut action, mut anim) = query
		.get_mut(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	let end = Dir2::from_rng(&mut rand::thread_rng());
	anim.curve = easing_curve(action.last_dir, end, action.func).into();
	action.last_dir = end;
}
