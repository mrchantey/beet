use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;


/// Updates the curve of [`PlayProceduralAnimation`] with a random direction curve
/// whenever an [`OnRun`] trigger is received.
#[derive(Debug, Clone, PartialEq, Component, Reflect, Action)]
#[observers(insert_procedural_animation)]
#[reflect(Default, Component)]
#[require(PlayProceduralAnimation)]
pub struct InsertProceduralAnimation {
	pub func: EaseFunction,
}

impl Default for InsertProceduralAnimation {
	fn default() -> Self {
		Self {
			func: EaseFunction::CubicInOut,
		}
	}
}

fn insert_procedural_animation(
	trigger: Trigger<OnRun>,
	transforms: Query<&Transform>,
	mut query: Query<(
		&TargetAgent,
		&InsertProceduralAnimation,
		&mut PlayProceduralAnimation,
	)>,
) {
	let (agent, action, mut anim) = query
		.get_mut(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	let transform = transforms
		.get(**agent)
		.expect(expect_action::TARGET_MISSING);

	// let local_pos = transform.inver

	let start =
		Dir2::new(transform.translation.xy()).unwrap_or_else(|_| Dir2::X);

	let end = Dir2::from_rng(&mut rand::thread_rng());
	anim.curve = easing_curve(start, end, action.func).into();
	// action.last_dir = end;
}
