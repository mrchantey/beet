//! Convert action failures to application exit codes.
use crate::prelude::*;
use beet_core::prelude::*;

/// Converts an [`Outcome::Fail`] into an [`AppExit::error()`].
///
/// Unlike [`ExitOnEnd`], this only triggers an exit on failure.
/// [`Outcome::Pass`] is ignored.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = World::new();
/// # world.insert_resource(Messages::<AppExit>::default());
/// world
///     .spawn((EndWith(Outcome::Fail), ExitOnFail))
///     .trigger_target(GetOutcome);
/// ```
#[action(exit_on_fail)]
#[derive(Debug, Component)]
pub struct ExitOnFail;

fn exit_on_fail(ev: On<Outcome>, mut commands: Commands) {
	match ev.event() {
		Outcome::Pass => {}
		Outcome::Fail => {
			commands.write_message(AppExit::error());
		}
	};
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
			.spawn((EndWith(Outcome::Fail), ExitOnFail))
			.trigger_target(GetOutcome)
			.flush();

		world.should_exit().unwrap().xpect_eq(AppExit::error());
	}
}
