use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::Interval;
use std::f32::consts::FRAC_PI_2;
use std::ops::Range;

/// Updates the curve of [`PlayProceduralAnimation`] with a random direction curve
/// whenever an [`OnRun`] trigger is received.
#[action(set_curve_on_run)]
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(PlayProceduralAnimation)]
pub enum SetCurveOnRun {
	/// Create a [`SerdeCurve::EaseDir2`]. The `from` position is the `xy` component of the target agent's [`Transform::translation`].
	EaseRangeDir2 {
		/// The range of angles to animate between.
		range: Range<f32>,
		/// The easing function to use.
		func: EaseFunction,
	},
	/// Three step animation, with `In`, `Pause` and `Out` phases.
	PingPongPause {
		/// The target position to animate to.
		target: Vec3,
		/// The length of the `Pause` relative to the `In` and `Out` animations.
		pause: f32,
		/// The easing function to use.
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
	ev: On<GetOutcome>,
	transforms: AgentQuery<&Transform>,
	mut rng: ResMut<RandomSource>,
	mut query: Query<(&SetCurveOnRun, &mut PlayProceduralAnimation)>,
) -> Result {
	let (action, mut anim) = query.get_mut(ev.action())?;

	let transform = transforms.get(ev.action())?;

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
	};
	Ok(())
}
