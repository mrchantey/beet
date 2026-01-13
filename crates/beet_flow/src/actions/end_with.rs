use crate::prelude::*;
use beet_core::prelude::*;

/// Immediately return a provided value when [`Run`] is called,
/// regardless of the world state.
/// This is conceptually similar to a `const` variable, although
/// the value can technically can be updated by some external system.
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
/// ## Example
/// returns `Outcome::Pass` when triggered.
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// World::new()
/// 	.spawn(EndWith(Outcome::Pass))
/// 	.trigger_target(GetOutcome);
/// ```
#[action(end_with::<T>)]
#[derive(Debug, Component, PartialEq, Eq)]
pub struct EndWith<T: EndEvent + Clone = Outcome>(pub T);

impl<T: EndEvent + Clone> EndWith<T> {}

fn end_with<T: EndEvent + Clone>(
	ev: On<T::Run>,
	mut commands: Commands,
	action: Query<&EndWith<T>>,
) -> Result {
	let target = ev.target();
	let action = action.get(target)?;
	commands.entity(target).trigger_target(action.0.clone());
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();

		let observed = observer_ext::observe_triggers::<Outcome>(&mut world);
		world
			.spawn(EndWith(Outcome::Pass))
			.trigger_target(GetOutcome)
			.flush();

		observed.len().xpect_eq(1);
		observed.get_index(0).unwrap().xpect_eq(Outcome::Pass);
	}
}
