//! `ApplyHeading`: the agent chooses where to head, recorded as a [`Heading`] component.
use beet_core::prelude::*;

/// Set the direction the body should head, based on what you can see.
///
/// The agent's `apply-heading` tool and the mock body's handler: logs the choice and
/// records the [`Heading`] on the caller (read it with `Single<&Heading>`). The wgpu body
/// (v2) serves the same route with `DriveFox` instead, mapping the heading onto the fox's
/// `DifferentialDrive` so the on-screen fox (and later the robot) moves.
#[action(route = "apply-heading")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn ApplyHeading(cx: ActionContext<ApplyHeadingInput>) -> Result<()> {
	let heading = cx.input.heading;
	info!("driving: {heading:?}");
	cx.caller.insert(heading).await?;
	Ok(())
}

/// What heading to apply.
#[derive(Reflect, serde::Deserialize, serde::Serialize)]
pub struct ApplyHeadingInput {
	/// The direction to head next. Prefer `Forward`, turning `Left` or `Right` only
	/// to avoid an obstacle.
	pub heading: Heading,
}

/// The direction the body is currently headed, set by [`ApplyHeading`] and read with
/// `Single<&Heading>`.
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	Component,
	Reflect,
	serde::Deserialize,
	serde::Serialize,
)]
#[reflect(Component, Default)]
pub enum Heading {
	/// Drive straight ahead.
	#[default]
	Forward,
	/// Turn to the left.
	Left,
	/// Turn to the right.
	Right,
}

#[cfg(test)]
mod test {
	use super::*;
	use beet_action::prelude::*;

	#[beet_core::test]
	async fn records_heading() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(ApplyHeading).id();
		world
			.entity_mut(entity)
			.call::<ApplyHeadingInput, ()>(ApplyHeadingInput {
				heading: Heading::Left,
			})
			.await
			.unwrap();
		world
			.entity(entity)
			.get::<Heading>()
			.copied()
			.xpect_eq(Some(Heading::Left));
	}
}
