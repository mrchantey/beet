//! The environment-agnostic drive leaf.
use crate::prelude::*;
use beet_core::prelude::*;

/// `<Drive linear=.. angular=..>` — the environment-agnostic motor leaf. On run it
/// writes its `(linear, angular)` onto the *agent's* persistent [`LinearVelocity`] +
/// [`AngularVelocity`] (resolved through [`AgentQuery`]), logs the step, then passes.
///
/// `linear` is mm/s, `angular` is deg/s (the typed units), so the same numbers mean
/// the same commanded motion in every environment. Pair with an [`EndInDuration`] in
/// a [`Sequence`] to "drive like this for N seconds" — the velocity persists on the
/// agent across the dwell, so the body keeps moving until the next step overrides it.
///
/// The leaf owns no body of its own: each environment reads the one agent velocity
/// for the body it owns (the wgpu `CharacterDrive` integrates it into a `Transform`,
/// the Alvik firmware maps it to the wheels), so the *same* `<Drive>` square patrol
/// runs headless, on-screen, and on the robot.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(DriveAction)]
pub struct Drive {
	/// Forward speed, mm/s (negative = reverse).
	pub linear: f32,
	/// Turn rate, deg/s (positive = left).
	pub angular: f32,
}

impl Drive {
	/// A drive at `linear` mm/s and `angular` deg/s.
	pub fn new(linear: f32, angular: f32) -> Self { Self { linear, angular } }
}

/// The action behind [`Drive`]: reads the caller's [`Drive`], logs the step, and
/// applies it to the agent's [`LinearVelocity`]/[`AngularVelocity`] (whichever the
/// agent carries), then passes. A headless tree whose agent has no velocity simply
/// logs — the body components are what an environment adds to opt its agent in.
#[action(handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub fn DriveAction(
	cx: In<ActionContext>,
	drives: Query<&Drive>,
	mut agents: AgentQuery<(
		Option<&'static mut LinearVelocity>,
		Option<&'static mut AngularVelocity>,
	)>,
) -> Outcome {
	if let Ok(drive) = drives.get(cx.id()) {
		info!("Drive: linear={} angular={}", drive.linear, drive.angular);
		if let Ok((linear, angular)) = agents.get_mut(cx.id()) {
			if let Some(mut linear) = linear {
				*linear = LinearVelocity::from_mm_per_sec(drive.linear);
			}
			if let Some(mut angular) = angular {
				*angular = AngularVelocity::from_deg_per_sec(drive.angular);
			}
		}
	}
	Outcome::PASS
}
