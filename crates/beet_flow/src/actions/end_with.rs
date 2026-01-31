//! Immediate return actions for constant values.
use crate::prelude::*;
use beet_core::prelude::*;

/// Immediately returns a constant value when triggered.
///
/// This action responds to any [`RunEvent`] by triggering the corresponding
/// [`EndEvent`] with the stored value. It is conceptually similar to a `const`
/// variable, though the value could be modified by external systems.
///
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
///
/// # Example
///
/// Returns [`Outcome::Pass`] when [`GetOutcome`] is triggered:
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = World::new();
/// world
///     .spawn(EndWith(Outcome::Pass))
///     .trigger_target(GetOutcome);
/// ```
///
/// Can also return scores for utility AI:
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = World::new();
/// world
///     .spawn(EndWith(Score::PASS))
///     .trigger_target(GetScore);
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
