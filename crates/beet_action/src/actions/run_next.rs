//! State-machine style jump to another action entity.
use crate::prelude::*;
use beet_core::prelude::*;

/// Calls another action entity, even outside this hierarchy.
///
/// In control-flow terms this is a [`goto`](https://xkcd.com/292/), useful
/// for state machines. The input is the preceding [`Outcome`]; when
/// `if_result_matches` is set and does not match, the jump is skipped and the
/// input is returned unchanged.
///
/// # Example
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// # let mut world = AsyncPlugin::world();
/// let next = world.spawn(EndWith(Outcome::PASS)).id();
/// world.spawn((EndWith(Outcome::PASS), RunNext::new(next)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[require(RunNextAction)]
#[reflect(Component)]
pub struct RunNext {
	/// The next action entity to call.
	pub target: Entity,
	/// If set, only jump when the input [`Outcome`] matches.
	pub if_result_matches: Option<Outcome>,
}

impl RunNext {
	/// Always jump to `target`.
	pub fn new(target: Entity) -> Self {
		Self {
			target,
			if_result_matches: None,
		}
	}
	/// Only jump when the input is [`Outcome::PASS`].
	pub fn if_success(target: Entity) -> Self {
		Self {
			target,
			if_result_matches: Some(Outcome::PASS),
		}
	}
	/// Only jump when the input is [`Outcome::FAIL`].
	pub fn if_failure(target: Entity) -> Self {
		Self {
			target,
			if_result_matches: Some(Outcome::FAIL),
		}
	}
}

/// Calls [`RunNext::target`] when the predicate matches, returning its
/// [`Outcome`]; otherwise returns the input unchanged.
///
/// ## Errors
/// Errors if the caller has no [`RunNext`] component.
#[action(default)]
#[derive(Component)]
pub async fn RunNextAction(cx: ActionContext<Outcome>) -> Result<Outcome> {
	let run_next = cx.caller.get_cloned::<RunNext>().await?;
	if let Some(expected) = run_next.if_result_matches {
		if cx.input != expected {
			return cx.input.xok();
		}
	}
	cx.world()
		.entity(run_next.target)
		.call::<(), Outcome>(())
		.await
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	async fn jumps_to_target() {
		let mut world = AsyncPlugin::world();
		let next = world.spawn(EndWith(Outcome::FAIL)).id();
		world
			.spawn(RunNext::new(next))
			.call::<Outcome, Outcome>(Outcome::PASS)
			.await
			.unwrap()
			.xpect_eq(Outcome::FAIL);
	}

	#[beet_core::test]
	async fn skips_when_predicate_unmatched() {
		let mut world = AsyncPlugin::world();
		let next = world.spawn(EndWith(Outcome::FAIL)).id();
		world
			.spawn(RunNext::if_success(next))
			.call::<Outcome, Outcome>(Outcome::FAIL)
			.await
			.unwrap()
			.xpect_eq(Outcome::FAIL);
	}
}
