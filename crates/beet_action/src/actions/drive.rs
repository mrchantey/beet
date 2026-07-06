//! The environment-agnostic drive leaves — [`SetDrive`] (set a velocity) and
//! [`DriveForDuration`] (drive for a bounded step) — and the [`DifferentialDrive`] command
//! they write onto the agent.
use crate::prelude::*;
use beet_core::prelude::*;

/// The commanded motion of a driven body: a forward speed plus a turn rate.
///
/// Declared on the body (the agent) at spawn — the wgpu `CharacterDrive`, the Alvik
/// robot root — and written by the [`SetDrive`] leaf through [`AgentQuery`]. Each
/// environment reads this one component for the body it owns (the fox integrates it
/// into a `Transform`, the Alvik maps it to its wheels), so the *same* `<SetDrive>`
/// square patrol runs headless, on-screen, and on the robot.
#[derive(Debug, Default, Clone, Copy, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DifferentialDrive {
	/// Forward speed (negative = reverse).
	pub linear: LinearVelocity,
	/// Turn rate (positive = left).
	pub angular: AngularVelocity,
}

impl DifferentialDrive {
	/// A command at the given linear + angular velocity.
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

/// Drive at a commanded differential velocity for a fixed duration, then stop — the
/// unit act the perceive-act body performs; the agent chooses the velocities
/// (direction + turn angle) and how long to hold them.
///
/// The shared wire type between the agent and whichever body serves the `drive`
/// capability (the wgpu fox, the Alvik robot): the agent serializes one of these onto
/// the route, and the body drives its [`DifferentialDrive`] for [`duration`](Self::duration)
/// before zeroing. [`duration`](Self::duration) serializes as a unit-suffixed string (via
/// [`duration_str`](beet_core::prelude::duration_str)) — eg `"1.5s"` — matching how a
/// [`Duration`] is authored in markup and shown in the model's tool schema, so the model
/// emits a value it reads straight from the schema.
///
/// Requires [`DriveForDurationAction`], so authoring a `<DriveForDuration/>` on a behavior
/// leaf drives the agent for the duration then stops without a separate action tag.
#[derive(Debug, Default, Clone, Copy, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
#[require(DriveForDurationAction)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DriveForDuration {
	/// The commanded velocity to hold.
	pub drive: DifferentialDrive,
	/// How long to hold the velocity before stopping, as a unit-suffixed duration
	/// string like `"1.5s"` or `"800ms"`.
	#[cfg_attr(feature = "serde", serde(with = "beet_core::prelude::duration_str"))]
	pub duration: Duration,
}

/// The action behind [`DriveForDuration`]: drive the agent at the commanded velocity for the
/// commanded duration, then stop.
///
/// Reads its caller's [`DriveForDuration`], resolves the driven agent through [`AgentQuery`]
/// (as [`SetDriveAction`] does), and runs a `SetDrive` + [`EndInDuration`] + `SetDrive(0, 0)`
/// sequence on it — the same "drive this velocity for N seconds then halt" step the perceive-act
/// wgpu body and the esp robot perform. Spawned automatically by [`DriveForDuration`]'s
/// `#[require]`, so `<DriveForDuration/>` is a self-contained leaf.
///
/// ## Errors
/// Errors if the agent has no [`DifferentialDrive`] (via the inner `SetDrive`) — declare it on
/// the driven body at spawn.
#[action(handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn DriveForDurationAction(cx: ActionContext) -> Result<Outcome> {
	let DriveForDuration { drive, duration } =
		cx.caller.get_cloned::<DriveForDuration>().await?;
	let world = cx.world();
	// the agent this action drives, resolved the same way `SetDrive` resolves its own.
	let agent = AgentQuery::entity_async(&world, cx.id()).await;
	// drive the agent for the duration then stop, as an action of the agent so the inner
	// `SetDrive`s resolve back to it; despawned after so steps never stack.
	let step = world
		.spawn((ActionOf(agent), Sequence::<(), ()>::default(), children![
			SetDrive::new(drive.linear, drive.angular),
			EndInDuration::pass(duration),
			SetDrive::new(0., 0.),
		]))
		.await;
	step.call::<(), Outcome>(()).await?;
	step.despawn().await?;
	Outcome::PASS.xok()
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// `DriveForDuration` requires `DriveForDurationAction`, so running the leaf drives the
	/// agent's `DifferentialDrive` at the commanded velocity, then zeroes it once the
	/// duration elapses — a self-contained "drive for N then stop" step.
	#[beet_core::test]
	async fn drives_the_agent_then_stops() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin, ActionPlugin));
		// the agent carries the `DifferentialDrive` the action writes.
		let agent = app.world_mut().spawn(DifferentialDrive::default()).id();
		// `<DriveForDuration/>` alone: its `#[require]` brings the action.
		let action = app
			.world_mut()
			.spawn((ActionOf(agent), DriveForDuration {
				drive: DifferentialDrive::new(40., 0.),
				duration: Duration::from_millis(100),
			}))
			.id();
		app.world_mut().flush();
		// fire the run; it sets the drive, holds it, then zeroes.
		app.world_mut().entity_mut(action).run_async_local(
			|action| async move { action.call::<(), Outcome>(()).await.map(|_| ()) },
		);
		// the commanded velocity lands on the agent while the step runs
		app_ext::update_until(&mut app, move |world| {
			world
				.get::<DifferentialDrive>(agent)
				.is_some_and(|drive| drive.linear.as_mm_per_sec() > 0.)
		})
		.await
		.xpect_true();
		// then it zeroes once the duration elapses
		app_ext::update_until_timeout(
			&mut app,
			move |world| {
				world
					.get::<DifferentialDrive>(agent)
					.is_some_and(|drive| drive.linear.as_mm_per_sec() == 0.)
			},
			Duration::from_secs(2),
		)
		.await
		.xpect_true();
	}

	/// The `drive` capability wire shape, shared verbatim with the esp body: a nested
	/// `drive` velocity as plain numbers (mm/s, deg/s) and `duration` as a unit-suffixed
	/// string. Locked — the esp side deserializes these identical bytes, so the shape is pinned.
	#[cfg(feature = "json")]
	#[beet_core::test]
	fn drive_for_duration_wire_shape() {
		let command = DriveForDuration {
			drive: DifferentialDrive::new(40., -90.),
			duration: Duration::from_secs_f64(1.5),
		};
		let json = serde_json::to_string(&command).unwrap();
		json.xpect_eq(
			r#"{"drive":{"linear":40.0,"angular":-90.0},"duration":"1.5s"}"#
				.to_string(),
		);
		serde_json::from_str::<DriveForDuration>(&json)
			.unwrap()
			.xpect_eq(command);
	}

	/// The model authors `duration` against a string-typed tool schema (reflect renders
	/// `Duration` as a string), so a unit-suffixed string decodes — but a bare number, a
	/// JSON number or a unit-less string, is rejected: a unit must always be provided. This
	/// is the live model boundary.
	#[cfg(feature = "json")]
	#[beet_core::test]
	fn drive_duration_requires_a_unit() {
		let decode = |duration: &str| {
			serde_json::from_str::<DriveForDuration>(&format!(
				r#"{{"drive":{{"linear":0.0,"angular":0.0}},"duration":{duration}}}"#
			))
		};
		// unit-suffixed strings decode
		decode(r#""1.5s""#)
			.unwrap()
			.duration
			.xpect_eq(Duration::from_secs_f64(1.5));
		decode(r#""250ms""#)
			.unwrap()
			.duration
			.xpect_eq(Duration::from_millis(250));
		// a bare number is rejected, whether a JSON number or a unit-less string
		decode("1.5").is_err().xpect_true();
		decode(r#""2""#).is_err().xpect_true();
	}
}
