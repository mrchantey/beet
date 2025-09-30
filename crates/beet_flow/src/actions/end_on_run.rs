use crate::prelude::*;
use beet_core::prelude::*;

/// Immediately return a provided value when [`OnRun`] is called,
/// regardless of the world state.
/// As an analogy this is similar to a `const` variable, although
/// it technically can be changed by some external system.
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
/// ## Example
/// returns `RunResult::Success` when triggered.
/// ```
/// # use beet_flow::doctest::*;
/// # let mut world = world();
/// world
/// 	.spawn(EndOnRun(RunResult::Success))
/// 	.trigger(OnRun::local());
/// ```
#[action(end_on_run::<R,T, E>)]
#[derive(Debug, Component, PartialEq, Eq)]
pub struct EndOnRun<
	R: 'static + Send + Sync = (),
	T: 'static + Send + Sync + Clone = (),
	E: 'static + Send + Sync + Clone = (),
> {
	event: End<T, E>,
	phantom: std::marker::PhantomData<R>,
}

impl<
	R: 'static + Send + Sync,
	T: 'static + Send + Sync + Clone,
	E: 'static + Send + Sync + Clone,
> EndOnRun<R, T, E>
{
	pub fn new(event: End<T, E>) -> Self {
		Self {
			event,
			phantom: default(),
		}
	}
}


impl EndOnRun<(), (), ()> {
	/// Create a new [`EndOnRun`] with [`End::Success`]
	pub fn success() -> Self { Self::new(SUCCESS) }
	pub fn failure() -> Self { Self::new(FAILURE) }
}

fn end_on_run<
	R: 'static + Send + Sync,
	T: 'static + Send + Sync + Clone,
	E: 'static + Send + Sync + Clone,
>(
	ev: On<Run<R>>,
	mut commands: Commands,
	action: Query<&EndOnRun<T>>,
) -> Result {
	let entity = ev.trigger().event_target();
	let action = action.get(entity)?;
	commands.entity(entity).trigger_target(action.event.clone());
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
		world.spawn(EndOnRun::success()).trigger_target(RUN);

		observed.len().xpect_eq(1);
		observed.get_index(0).unwrap().xpect_eq(SUCCESS);
	}
}
