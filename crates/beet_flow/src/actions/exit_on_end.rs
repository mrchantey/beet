//! Convert action outcomes to application exit codes.
use crate::prelude::*;
use beet_core::prelude::*;

/// Converts an [`Outcome`] into an [`AppExit`].
///
/// When an [`Outcome`] is triggered on this entity:
/// - [`Outcome::Pass`] results in [`AppExit::Success`]
/// - [`Outcome::Fail`] results in [`AppExit::error()`]
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = World::new();
/// # world.insert_resource(Messages::<AppExit>::default());
/// world
///     .spawn((EndWith(Outcome::Pass), ExitOnEnd))
///     .trigger_target(GetOutcome);
/// ```
#[action(exit_on_end)]
#[derive(Debug, Component)]
pub struct ExitOnEnd;

fn exit_on_end(ev: On<Outcome>, mut commands: Commands) {
	let exit = match ev.event() {
		Outcome::Pass => AppExit::Success,
		Outcome::Fail => AppExit::error(),
	};
	commands.write_message(exit);
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();
		world.insert_resource(Messages::<AppExit>::default());
		world
			.spawn((EndWith(Outcome::Pass), ExitOnEnd))
			.trigger_target(GetOutcome)
			.flush();

		world.should_exit().unwrap().xpect_eq(AppExit::Success);
	}
}
