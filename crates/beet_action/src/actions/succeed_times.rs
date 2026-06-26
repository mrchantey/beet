//! Debugging leaf that succeeds a limited number of times.
use crate::prelude::*;
use beet_core::prelude::*;

/// Threads the input through as [`Outcome::Pass`] up to `max_times`, then
/// returns [`Outcome::Fail`] with the input.
///
/// A debugging utility, the run count is stored on the component. Generic
/// over the threaded value `T`, which defaults to `()`.
///
/// # Example
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// # let mut world = AsyncPlugin::world();
/// world.spawn(SucceedTimes::new(2));
/// ```
#[derive(Debug, Clone, Component, Reflect)]
#[require(SucceedTimesAction<T>)]
#[reflect(Component, Default)]
pub struct SucceedTimes<T = ()>
where
	T: 'static + Send + Sync + Clone,
{
	/// The number of times to succeed.
	pub max_times: u32,
	/// The number of times this action has been run.
	pub times: u32,
	#[reflect(ignore)]
	_marker: PhantomData<fn() -> T>,
}

impl SucceedTimes<()> {
	/// Create a new `SucceedTimes<()>` that passes `max_times` times.
	pub fn new(max_times: u32) -> Self { Self::typed(max_times) }
}

impl<T> SucceedTimes<T>
where
	T: 'static + Send + Sync + Clone,
{
	/// Create a typed [`SucceedTimes`] that passes `max_times` times.
	pub fn typed(max_times: u32) -> Self {
		Self {
			max_times,
			times: 0,
			_marker: PhantomData,
		}
	}
}

impl<T> Default for SucceedTimes<T>
where
	T: 'static + Send + Sync + Clone,
{
	fn default() -> Self { Self::typed(0) }
}

/// Increments the run count, threading the input as [`Outcome::Pass`] until
/// `max_times` is reached, then [`Outcome::Fail`].
///
/// ## Errors
/// Errors if the caller has no [`SucceedTimes`] component.
#[action(default)]
#[derive(Component)]
pub async fn SucceedTimesAction<T>(
	cx: ActionContext<T>,
) -> Result<Outcome<T, T>>
where
	T: 'static + Send + Sync + Clone,
{
	let passed = cx
		.caller
		.get_mut::<SucceedTimes<T>, _>(|mut action| {
			if action.times < action.max_times {
				action.times += 1;
				true
			} else {
				false
			}
		})
		.await?;
	if passed {
		Ok(Outcome::Pass(cx.input))
	} else {
		Ok(Outcome::Fail(cx.input))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	async fn passes_then_fails() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(SucceedTimes::new(2)).id();

		world
			.entity_mut(entity)
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		world
			.entity_mut(entity)
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
		world
			.entity_mut(entity)
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::FAIL);
	}

	#[beet_core::test]
	async fn threads_value() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(SucceedTimes::<i32>::typed(1)).id();
		world
			.entity_mut(entity)
			.call::<i32, Outcome<i32, i32>>(7)
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(7));
		world
			.entity_mut(entity)
			.call::<i32, Outcome<i32, i32>>(7)
			.await
			.unwrap()
			.xpect_eq(Outcome::Fail(7));
	}
}
