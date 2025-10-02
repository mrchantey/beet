use crate::prelude::*;
use beet_core::prelude::*;

/// Immediately return a provided value when [`Run`] is called,
/// regardless of the world state.
/// This is conceptually similar to a `const` variable, although
/// it technically can be changed by some external system.
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
/// ## Example
/// returns `SUCCESS` when triggered.
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// World::new()
/// 	.spawn(EndOnRun(SUCCESS))
/// 	.trigger_entity(RUN);
/// ```
#[action(end_on_run::<T>)]
#[derive(Debug, Component, PartialEq, Eq)]
pub struct EndOnRun<T: EndPayload + Clone = EndResult>(pub T);

impl<T: EndPayload + Clone> EndOnRun<T> {}

fn end_on_run<T: EndPayload + Clone>(
	ev: On<Run<T::Run>>,
	mut commands: Commands,
	action: Query<&EndOnRun<T>>,
) -> Result {
	let entity = ev.event_target();
	let action = action.get(entity)?;
	commands.entity(entity).trigger_entity(action.0.clone());
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();

		let observed = observer_ext::observe_triggers::<End>(&mut world);
		world.spawn(EndOnRun(SUCCESS)).trigger_entity(RUN).flush();

		observed.len().xpect_eq(1);
		observed.get_index(0).unwrap().value().xpect_eq(SUCCESS);
	}
}
