//! `DriveForDurationAction`: the mock body's `drive` handler, recording the commanded
//! [`DifferentialDrive`] for tests. The wgpu body (v2) serves the same `drive` route with
//! [`DriveFox`](super::DriveFox) instead, actually moving the on-screen fox.
use crate::beet::prelude::*;
use beet_core::prelude::*;

/// Drive at the commanded velocity for the commanded duration, then stop.
///
/// The agent's local `drive` fallback and the v1 mock body's handler: logs the command
/// and records the received [`DriveForDuration`] on the caller (read it with
/// `Single<&DriveForDuration>`) so tests can assert both the velocity and the (clamped)
/// duration the agent chose. The wgpu body (v2) serves the same route with
/// [`DriveFox`](super::DriveFox), which integrates the command into the fox's transform.
#[action(route = "drive")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn DriveForDurationAction(
	cx: ActionContext<DriveForDuration>,
) -> Result<()> {
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

	#[beet_core::test]
	async fn records_drive() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(DriveForDurationAction).id();
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
