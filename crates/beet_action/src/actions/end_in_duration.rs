//! Duration-delayed return action.
use crate::prelude::*;
use beet_core::prelude::*;

/// Waits for a duration, then returns the stored value.
///
/// Replaces the old `RunTimer` + `Running` pattern: a long-running action is
/// now just a future that resolves later. Pair with [`Fallback`] to race a
/// timeout against a child.
///
/// # Example
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// # let mut world = AsyncPlugin::world();
/// world.spawn(EndInDuration::pass(Duration::from_secs(2)));
/// ```
#[derive(Debug, Clone, Component, Reflect)]
#[require(EndInDurationAction<T>)]
#[reflect(Component)]
pub struct EndInDuration<T = Outcome>
where
	T: 'static + Send + Sync + Clone,
{
	/// How long to wait before returning.
	pub duration: Duration,
	/// The value to return once the duration elapses.
	pub value: T,
}

impl<T> EndInDuration<T>
where
	T: 'static + Send + Sync + Clone,
{
	/// Wait `duration`, then return `value`.
	pub fn new(value: T, duration: Duration) -> Self {
		Self { value, duration }
	}
}

impl<T> Default for EndInDuration<T>
where
	T: 'static + Send + Sync + Clone + Default,
{
	fn default() -> Self { Self::new(T::default(), Duration::from_secs(1)) }
}

impl EndInDuration<Outcome> {
	/// Return [`Outcome::PASS`] after `duration`.
	pub fn pass(duration: Duration) -> Self {
		Self::new(Outcome::PASS, duration)
	}
	/// Return [`Outcome::FAIL`] after `duration`.
	pub fn fail(duration: Duration) -> Self {
		Self::new(Outcome::FAIL, duration)
	}
}

/// Sleeps for the configured duration, then returns the stored value.
///
/// ## Errors
/// Errors if the caller has no [`EndInDuration`] component.
#[action(default)]
#[derive(Component)]
pub async fn EndInDurationAction<T>(cx: ActionContext) -> Result<T>
where
	T: 'static + Send + Sync + Clone,
{
	let action = cx.caller.get_cloned::<EndInDuration<T>>().await?;
	time_ext::sleep(action.duration).await;
	action.value.xok()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	async fn returns_after_delay() {
		AsyncPlugin::world()
			.spawn(EndInDuration::pass(Duration::from_millis(10)))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn fallback_races_timeout_against_slow_child() {
		// the slow child takes far longer than the fast sibling
		AsyncPlugin::world()
			.spawn((Fallback::new(), children![
				EndInDuration::fail(Duration::from_millis(5)),
				EndInDuration::pass(Duration::from_millis(1)),
			]))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
}
