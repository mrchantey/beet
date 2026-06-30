//! `SetHeading`: the agent chooses where to head, recorded as a [`Heading`] component.
use beet_core::prelude::*;

/// Set the direction the body should head, based on what you can see.
///
/// Records the chosen [`Heading`] on the caller; read it elsewhere with
/// `Single<&Heading>`. v1 only records it; v2 maps it onto the body's
/// `DifferentialDrive` so the on-screen fox (and later the robot) moves.
#[action(route = "set-heading")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn SetHeading(cx: ActionContext<SetHeadingInput>) -> Result<()> {
	let heading = cx.input.heading;
	info!("SetHeading: {heading:?}");
	cx.caller.insert(heading).await?;
	Ok(())
}

/// What heading to set.
#[derive(Reflect, serde::Deserialize, serde::Serialize)]
pub struct SetHeadingInput {
	/// The direction to head next. Prefer `Forward`, turning `Left` or `Right` only
	/// to avoid an obstacle.
	pub heading: Heading,
}

/// The direction the body is currently headed, set by [`SetHeading`] and read with
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
		let entity = world.spawn(SetHeading).id();
		world
			.entity_mut(entity)
			.call::<SetHeadingInput, ()>(SetHeadingInput {
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
