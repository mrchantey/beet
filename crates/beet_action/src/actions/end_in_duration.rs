//! Duration-delayed return action.
use crate::prelude::*;
use beet_core::prelude::*;
use core::marker::PhantomData;

/// Runs for a duration, then ends with the stored value.
///
/// A long-running action: [`ContinueRun`] keeps the call pending while the
/// [`end_in_duration`] system watches its [`RunTimer`]. Once `duration`
/// elapses the run is ended with `value`. Because it stays [`Running`] it can
/// be interrupted by another system removing [`Running`] first. Pair with
/// [`Fallback`] to race a timeout against a child.
///
/// # Example
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// # let mut world = (MinimalPlugins, AsyncPlugin, ActionPlugin).into_world();
/// world.spawn(EndInDuration::pass(Duration::from_secs(2)));
/// ```
#[derive(Debug, Clone, Component, Reflect)]
#[require(ContinueRun<In, Out>)]
#[reflect(Component)]
pub struct EndInDuration<In = (), Out = Outcome>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync + Clone,
{
	/// How long to run before ending.
	pub duration: Duration,
	/// The value to end with once the duration elapses.
	pub value: Out,
	#[reflect(ignore)]
	_marker: PhantomData<fn() -> In>,
}

impl<In, Out> EndInDuration<In, Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync + Clone,
{
	/// Run `duration`, then end with `value`.
	pub fn new(value: Out, duration: Duration) -> Self {
		Self {
			value,
			duration,
			_marker: PhantomData,
		}
	}
}

impl<In, Out> Default for EndInDuration<In, Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync + Clone + Default,
{
	fn default() -> Self { Self::new(Out::default(), Duration::from_secs(1)) }
}

impl EndInDuration<(), Outcome> {
	/// End with [`Outcome::PASS`] after `duration`.
	pub fn pass(duration: Duration) -> Self {
		Self::new(Outcome::PASS, duration)
	}
	/// End with [`Outcome::FAIL`] after `duration`.
	pub fn fail(duration: Duration) -> Self {
		Self::new(Outcome::FAIL, duration)
	}
}

/// Ends any [`Running`] [`EndInDuration`] whose [`RunTimer`] has reached its
/// configured duration.
pub(crate) fn end_in_duration<In, Out>(
	mut commands: Commands,
	query: Populated<
		(Entity, &RunTimer, &EndInDuration<In, Out>),
		With<Running<Out>>,
	>,
) where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync + Clone,
{
	for (entity, timer, action) in query.iter() {
		if timer.last_run.elapsed() >= action.duration {
			commands.entity(entity).queue(EndRun(action.value.clone()));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	async fn returns_after_delay() {
		(MinimalPlugins, AsyncPlugin, ActionPlugin)
			.into_world()
			.spawn(EndInDuration::pass(Duration::from_millis(10)))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn fallback_races_timeout_against_slow_child() {
		// the slow child takes far longer than the fast sibling
		(MinimalPlugins, AsyncPlugin, ActionPlugin)
			.into_world()
			.spawn((Fallback::new(), children![
				EndInDuration::fail(Duration::from_millis(20)),
				EndInDuration::pass(Duration::from_millis(1)),
			]))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
}
