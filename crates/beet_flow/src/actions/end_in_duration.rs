use crate::prelude::*;
use beet_core::prelude::*;

/// Triggers the given event after running for a given duration.
/// This has no effect if the action completes before the duration.
///
/// The default duration is 1 second.
/// ## Example
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = world();
/// world.spawn((
///		Running::default(),
///		EndInDuration::success(Duration::from_secs(2)),
///	));
///
/// ```
#[derive(Debug, Clone, Component, Reflect)]
#[require(ContinueRun)]
pub struct EndInDuration<T> {
	/// The length of time the action will run for before triggering the event.
	pub duration: Duration,
	/// The payload to return with
	pub event: T,
}

impl<T> Default for EndInDuration<T>
where
	T: Default,
{
	fn default() -> Self { Self::new(default(), Duration::from_secs(1)) }
}

impl<T> EndInDuration<T> {
	/// Specify the payload and duration
	pub fn new(event: T, duration: Duration) -> Self {
		Self { event, duration }
	}
}

impl EndInDuration<IntoEnd> {
	pub fn success(duration: Duration) -> Self {
		Self::new(IntoEnd::success(), duration)
	}
	pub fn failure(duration: Duration) -> Self {
		Self::new(IntoEnd::failure(), duration)
	}
}

pub(crate) fn end_in_duration<T: IntoEntityEvent + Clone>(
	mut commands: Commands,
	mut query: Populated<
		(Entity, &RunTimer, &mut EndInDuration<T>),
		With<Running>,
	>,
) {
	for (entity, timer, action) in query.iter_mut() {
		if timer.last_run.elapsed() >= action.duration {
			commands.entity(entity).trigger_entity(action.event.clone());
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default()).insert_time();

		let on_result = observer_ext::observe_triggers::<End>(app.world_mut());

		app.world_mut().spawn((
			Running::default(),
			EndInDuration::success(Duration::from_secs(2)),
		));

		app.update_with_secs(1);

		on_result.is_empty().xpect_true();
		app.update_with_secs(10);
		on_result.is_empty().xpect_false();
	}
}
