use crate::prelude::*;
use beet_core::prelude::*;

/// Convert an [`End<Outcome>`] into an [`AppExit`]
#[action(exit_on_end)]
#[derive(Debug, Component)]
pub struct ExitOnEnd;

fn exit_on_end(ev: On<End>, mut commands: Commands) {
	let exit = match ev.value() {
		Outcome::Pass => AppExit::Success,
		Outcome::Fail => AppExit::error(),
	};
	commands.write_message(exit);
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();
		world.insert_resource(Messages::<AppExit>::default());
		world
			.spawn((EndWith(Outcome::Pass), ExitOnEnd))
			.trigger_action(GetOutcome)
			.flush();

		world.should_exit().unwrap().xpect_eq(AppExit::Success);
	}
}
