//! Debugging leaf that succeeds a limited number of times.
use crate::prelude::*;
use beet_core::prelude::*;

/// Returns [`Outcome::PASS`] up to `max_times`, then [`Outcome::FAIL`].
///
/// A debugging utility, the run count is stored on the component.
///
/// # Example
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// # let mut world = AsyncPlugin::world();
/// world.spawn(SucceedTimes::new(2));
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[require(SucceedTimesAction)]
#[reflect(Default, Component)]
pub struct SucceedTimes {
	/// The number of times to succeed.
	pub max_times: u32,
	/// The number of times this action has been run.
	pub times: u32,
}

impl SucceedTimes {
	/// Create a new [`SucceedTimes`] that passes `max_times` times.
	pub fn new(max_times: u32) -> Self {
		Self {
			max_times,
			times: 0,
		}
	}
}

/// Increments the run count, passing until `max_times` is reached.
///
/// ## Errors
/// Errors if the caller has no [`SucceedTimes`] component.
#[action(default)]
#[derive(Component)]
pub async fn SucceedTimesAction(cx: ActionContext) -> Result<Outcome> {
	cx.caller
		.get_mut::<SucceedTimes, _>(|mut action| {
			if action.times < action.max_times {
				action.times += 1;
				Outcome::PASS
			} else {
				Outcome::FAIL
			}
		})
		.await
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
}
