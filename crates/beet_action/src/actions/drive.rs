//! The environment-agnostic drive leaf and the commanded velocity it writes.
use crate::prelude::*;
use beet_core::prelude::*;

/// The commanded motion of a driven body: a forward speed plus a turn rate.
///
/// Declared on the body (the agent) at spawn — the wgpu `CharacterDrive`, the Alvik
/// robot root — and written by the [`SetDrive`] leaf through [`AgentQuery`]. Each
/// environment reads this one component for the body it owns (the fox integrates it
/// into a `Transform`, the Alvik maps it to its wheels), so the *same* `<SetDrive>`
/// square patrol runs headless, on-screen, and on the robot.
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
pub struct DifferentialDrive {
	/// Forward speed (negative = reverse).
	pub linear: LinearVelocity,
	/// Turn rate (positive = left).
	pub angular: AngularVelocity,
}

/// `<SetDrive linear=.. angular=..>` — the environment-agnostic motor leaf. On run it
/// writes its `(linear, angular)` onto the agent's [`DifferentialDrive`] (resolved
/// through [`AgentQuery`]), logs the step, then passes. `linear` is mm/s, `angular`
/// is deg/s, so the same numbers mean the same commanded motion everywhere. Pair with
/// an [`EndInDuration`] in a [`Sequence`] to "drive like this for N seconds" — the
/// command persists on the agent across the dwell.
///
/// The agent must carry a [`DifferentialDrive`], declared on the body at spawn; the
/// action errors loudly if it is missing rather than silently doing nothing.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(SetDriveAction)]
pub struct SetDrive {
	/// Forward speed, mm/s (negative = reverse).
	pub linear: LinearVelocity,
	/// Turn rate, deg/s (positive = left).
	pub angular: AngularVelocity,
}

impl SetDrive {
	/// A drive at the given linear + angular velocity.
	pub fn new(
		linear: impl Into<LinearVelocity>,
		angular: impl Into<AngularVelocity>,
	) -> Self {
		Self {
			linear: linear.into(),
			angular: angular.into(),
		}
	}
}

/// The action behind [`SetDrive`]: reads the caller's [`SetDrive`], logs the step, and
/// applies it to the agent's [`DifferentialDrive`], then passes.
///
/// ## Errors
/// Errors if the agent has no [`DifferentialDrive`] — declare it on the driven body
/// at spawn (the wgpu `CharacterDrive` requires it, the Alvik root spawns with it).
#[action(handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub fn SetDriveAction(
	cx: In<ActionContext>,
	drives: Query<&SetDrive>,
	mut agents: AgentQuery<&'static mut DifferentialDrive>,
) -> Result<Outcome> {
	let drive = drives.get(cx.id())?;
	info!(
		"SetDrive: linear={} angular={}",
		drive.linear.as_mm_per_sec(),
		drive.angular.as_deg_per_sec()
	);
	let mut command = agents.get_mut(cx.id()).map_err(|_| {
		bevyhow!(
			"SetDrive action {}: its agent has no `DifferentialDrive` component — \
			declare it on the driven body at spawn",
			cx.id()
		)
	})?;
	command.linear = drive.linear;
	command.angular = drive.angular;
	Outcome::PASS.xok()
}
