//! `Drive`: the agent picks a heading. Mocked in v1 (logs and records the heading).
use beet_core::prelude::*;

/// Drive the body in a direction. Choose where to go based on what you can see.
///
/// v1 records the heading on [`LastHeading`] and logs it; v2 maps it onto the body's
/// `DifferentialDrive` so the on-screen fox (and later the robot) actually moves.
#[action(route = "drive")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn Drive(cx: ActionContext<DriveInput>) -> Result<()> {
	let heading = cx.input.heading;
	info!("Drive (mock): {heading:?}");
	cx.caller
		.with_state::<ResMut<LastHeading>, _>(move |_entity, mut last| {
			last.0 = Some(heading);
		})
		.await?;
	Ok(())
}

/// A direction for the body to head.
#[derive(
	Debug, Clone, Copy, PartialEq, Reflect, serde::Deserialize, serde::Serialize,
)]
pub enum Heading {
	/// Drive straight ahead.
	Forward,
	/// Turn to the left.
	Left,
	/// Turn to the right.
	Right,
}

/// What heading to drive.
#[derive(Reflect, serde::Deserialize, serde::Serialize)]
pub struct DriveInput {
	/// The direction to head next. Prefer `Forward`, turning `Left` or `Right` only
	/// to avoid an obstacle.
	pub heading: Heading,
}

/// The heading the agent last chose, recorded by [`Drive`]. v1 has no body, so this
/// is a drive's observable result; v2 reads it to move the body.
#[derive(Debug, Default, Clone, Resource)]
pub struct LastHeading(pub Option<Heading>);

#[cfg(test)]
mod test {
	use super::*;
	use beet_action::prelude::*;

	#[beet_core::test]
	async fn records_heading() {
		let mut world = AsyncPlugin::world();
		world.insert_resource(LastHeading::default());
		let entity = world.spawn(Drive).id();
		world
			.entity_mut(entity)
			.call::<DriveInput, ()>(DriveInput {
				heading: Heading::Left,
			})
			.await
			.unwrap();
		world.resource::<LastHeading>().0.xpect_eq(Some(Heading::Left));
	}
}
