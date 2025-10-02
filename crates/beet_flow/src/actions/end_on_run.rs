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
/// 	.spawn(EndOnRun::success())
/// 	.trigger_target(RUN);
/// ```
#[action(end_on_run::<R,E>)]
#[derive(Debug, Component, PartialEq, Eq)]
pub struct EndOnRun<R = (), E = IntoEnd>
where
	R: 'static + Send + Sync,
	E: IntoEntityEvent + Clone,
{
	event: E,
	phantom: std::marker::PhantomData<R>,
}

impl<R, E> EndOnRun<R, E>
where
	R: 'static + Send + Sync,
	E: IntoEntityEvent + Clone,
{
	pub fn new(event: E) -> Self {
		Self {
			event,
			phantom: default(),
		}
	}
}

impl EndOnRun<(), IntoEnd> {
	/// Create a new [`EndOnRun`] with [`End::Success`]
	pub fn success() -> Self { Self::new(IntoEnd::success()) }
	pub fn failure() -> Self { Self::new(IntoEnd::failure()) }
}

fn end_on_run<R, E>(
	ev: On<Run<R>>,
	mut commands: Commands,
	action: Query<&EndOnRun<R, E>>,
) -> Result
where
	R: 'static + Send + Sync,
	E: IntoEntityEvent + Clone,
{
	let entity = ev.event_target();
	let action = action.get(entity)?;
	commands.entity(entity).trigger_entity(action.event.clone());
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
		world.spawn(EndOnRun::success()).trigger_entity(RUN).flush();

		observed.len().xpect_eq(1);
		observed.get_index(0).unwrap().value().xpect_ok();
	}
}
