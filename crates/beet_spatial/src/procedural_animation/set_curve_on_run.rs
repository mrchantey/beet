use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;
use std::ops::Range;
use sweet::prelude::RandomSource;

/// Updates the curve of [`PlayProceduralAnimation`] with a random direction curve
/// whenever an [`OnRun`] trigger is received.
#[derive(Debug, Clone, PartialEq, Component, Reflect, Action)]
#[observers(set_curve_on_run)]
#[reflect(Default, Component)]
#[require(PlayProceduralAnimation)]
pub enum SetCurveOnRun {
	/// Create a [`SerdeCurve::EaseDir2`]. The `from` position is the `xy` component of the target agent's [`Transform::translation`].
	EaseRangeDir2 {
		range: Range<f32>,
		func: EaseFunction,
	},
	/// Three step animation, with `In`, `Pause` and `Out` phases.
	PingPongPause {
		target: Vec3,
		/// The length of the `Pause` relative to the `In` and `Out` animations.
		pause: f32,
		func: EaseFunction,
	},
}

impl Default for SetCurveOnRun {
	fn default() -> Self {
		Self::EaseRangeDir2 {
			range: -FRAC_PI_2..FRAC_PI_2,
			func: EaseFunction::CubicInOut,
		}
	}
}

impl SetCurveOnRun {}

fn set_curve_on_run(
	trigger: Trigger<OnRun>,
	transforms: Query<&Transform>,
	mut rng: ResMut<RandomSource>,
	mut query: Query<(
		&TargetEntity,
		&SetCurveOnRun,
		&mut PlayProceduralAnimation,
	)>,
) {
	let (agent, action, mut anim) = query
		.get_mut(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	let transform = transforms
		.get(**agent)
		.expect(expect_action::TARGET_MISSING);

	anim.curve = match action {
		SetCurveOnRun::EaseRangeDir2 { func, range } => {
			let start = Dir2::new(transform.translation.xy())
				.unwrap_or_else(|_| Dir2::X);

			let angle =
				range.start + (range.end - range.start) * rng.random::<f32>();
			let end = Dir2::new_unchecked(Vec2::new(angle.cos(), angle.sin()));

			EasingCurve::new(start, end, *func).into()
		}
		SetCurveOnRun::PingPongPause {
			target,
			func,
			pause,
		} => {
			let start = transform.translation;

			let from = EasingCurve::new(start, *target, *func);
			let pause =
				FunctionCurve::new(Interval::new(0., *pause).unwrap(), |_| {
					*target
				});
			let to = EasingCurve::new(*target, start, *func);

			from.chain(pause)
				.unwrap()
				.chain(to)
				.unwrap()
				.reparametrize_linear(Interval::UNIT)
				.unwrap()
				.resample_auto(32)
				.unwrap()
				.into()
		}
	}
}
