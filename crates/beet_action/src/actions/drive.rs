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
/// before zeroing. [`duration`](Self::duration) serializes as a unit-suffixed string (see
/// [`duration_str`]) — eg `"1.5s"` — matching how a [`Duration`] is authored in markup and
/// shown in the model's tool schema, so the model emits a value it reads straight from the schema.
#[derive(Debug, Default, Clone, Copy, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DriveForDuration {
	/// The commanded velocity to hold.
	pub drive: DifferentialDrive,
	/// How long to hold the velocity before stopping, as a duration string like
	/// `"1.5s"` or `"800ms"`.
	#[cfg_attr(feature = "serde", serde(with = "duration_str"))]
	pub duration: Duration,
}

/// Serde for [`DriveForDuration::duration`]. Serializes as a unit-suffixed string (eg
/// `"1.5s"`), matching how the reflect tool schema presents a [`Duration`] to the model (a
/// string, coerced from `"30s"`-style markup) — so the model emits a value the deserializer
/// accepts. Deserialization is lenient: a unit-suffixed string (`"250ms"`), a bare numeric
/// string (`"1.5"` = seconds) or a raw JSON number (`1.5` = seconds) all decode; a
/// non-finite or negative value decodes to [`Duration::ZERO`] rather than panicking.
#[cfg(feature = "serde")]
mod duration_str {
	use beet_core::prelude::*;
	use core::fmt;
	use serde::Deserializer;
	use serde::Serializer;
	use serde::de::Visitor;

	pub fn serialize<S>(
		duration: &Duration,
		serializer: S,
	) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&format!("{}s", duration.as_secs_f64()))
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_any(DurationVisitor)
	}

	/// A finite, non-negative seconds value, else [`Duration::ZERO`].
	fn secs_to_duration(secs: f64) -> Duration {
		Duration::try_from_secs_f64(secs).unwrap_or(Duration::ZERO)
	}

	struct DurationVisitor;

	impl<'de> Visitor<'de> for DurationVisitor {
		type Value = Duration;

		fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
			f.write_str("a duration string like \"1.5s\" or a number of seconds")
		}

		fn visit_str<E>(self, value: &str) -> Result<Duration, E>
		where
			E: serde::de::Error,
		{
			// prefer a unit-suffixed string, fall back to bare seconds
			Duration::from_human_str(value)
				.or_else(|| {
					value.trim().parse::<f64>().ok().map(secs_to_duration)
				})
				.ok_or_else(|| E::custom(format!("invalid duration: {value:?}")))
		}

		fn visit_f64<E>(self, value: f64) -> Result<Duration, E>
		where
			E: serde::de::Error,
		{
			Ok(secs_to_duration(value))
		}

		fn visit_u64<E>(self, value: u64) -> Result<Duration, E>
		where
			E: serde::de::Error,
		{
			Ok(secs_to_duration(value as f64))
		}

		fn visit_i64<E>(self, value: i64) -> Result<Duration, E>
		where
			E: serde::de::Error,
		{
			Ok(secs_to_duration(value as f64))
		}
	}
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

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The `drive` capability wire shape, shared verbatim with the esp body: a nested
	/// `drive` velocity as plain numbers (mm/s, deg/s) and `duration` as a unit-suffixed
	/// string. Locked — the esp side deserializes these identical bytes, so the shape is pinned.
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
	/// `Duration` as a string), so decoding tolerates a unit-suffixed string, a bare numeric
	/// string, or a raw number — all read as seconds. This is the live model boundary.
	#[beet_core::test]
	fn drive_duration_decodes_leniently() {
		let decode = |duration: &str| {
			serde_json::from_str::<DriveForDuration>(&format!(
				r#"{{"drive":{{"linear":0.0,"angular":0.0}},"duration":{duration}}}"#
			))
			.unwrap()
			.duration
		};
		decode(r#""1.5s""#).xpect_eq(Duration::from_secs_f64(1.5));
		decode(r#""250ms""#).xpect_eq(Duration::from_millis(250));
		decode(r#""2""#).xpect_eq(Duration::from_secs(2));
		decode("1.5").xpect_eq(Duration::from_secs_f64(1.5));
	}
}
