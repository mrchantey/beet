//! `LogDriveForDuration`: the perceive-act demo's mock `drive` handler, recording the commanded
//! [`DriveForDuration`] for tests rather than moving anything. The agent's local `drive`
//! fallback and the v1 mock body both serve `drive` with it; a real body drives instead —
//! the wgpu body (v2) via [`DriveFox`](super::DriveFox), the esp robot via its own handler,
//! both applying the same [`DriveForDuration`] through the canonical `DriveForDurationAction`.
use crate::beet::prelude::*;
use beet_core::prelude::*;

/// Record the commanded `drive` for tests, without a body to move.
///
/// The agent's local `drive` fallback and the v1 mock body's handler: logs the command and
/// records the received [`DriveForDuration`] on the caller (read it with
/// `Single<&DriveForDuration>`) so tests can assert both the velocity and the (clamped)
/// duration the agent chose. A real body applies the command instead: the wgpu body (v2)
/// through [`DriveFox`](super::DriveFox), the esp robot through its own handler — both drive
/// the shared [`DriveForDuration`] via the canonical `DriveForDurationAction`.
#[action(route = "drive")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn LogDriveForDuration(cx: ActionContext<DriveForDuration>) -> Result<()> {
	let command = cx.input;
	info!(
		"drive: lin={} ang={} for {:.2}s",
		command.drive.linear.as_mm_per_sec(),
		command.drive.angular.as_deg_per_sec(),
		command.duration.as_secs_f32()
	);
	cx.caller.insert(command).await?;
	Ok(())
}

#[cfg(test)]
mod test {
	use super::*;
	use beet_action::prelude::*;

	/// Serving `drive` records the received [`DriveForDuration`] on the caller, so a test
	/// can assert the velocity and duration the agent chose without a body to move.
	#[beet_core::test]
	async fn records_drive() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(LogDriveForDuration).id();
		let command = DriveForDuration {
			drive: DifferentialDrive::new(40., 90.),
			duration: Duration::from_secs(1),
		};
		world
			.entity_mut(entity)
			.call::<DriveForDuration, ()>(command)
			.await
			.unwrap();
		world
			.entity(entity)
			.get::<DriveForDuration>()
			.copied()
			.xpect_eq(Some(command));
	}
}
