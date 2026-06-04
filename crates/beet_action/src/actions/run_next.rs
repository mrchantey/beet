//! State-machine style jump to another action entity.
use crate::prelude::*;
use beet_core::prelude::*;

/// Calls another action entity, even outside this hierarchy.
///
/// In control-flow terms this is a [`goto`](https://xkcd.com/292/), useful
/// for state machines. The threaded value `T` is passed to the target and
/// its result returned. When `if_input_matches` is set and does not equal
/// the input, the jump is skipped and the input is returned unchanged.
///
/// `T` defaults to [`Outcome`], so a bare `RunNext` threads an [`Outcome`].
///
/// # Example
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// # let mut world = AsyncPlugin::world();
/// let next = world
/// 	.spawn(Action::<Outcome, Outcome>::new_pure(|cx: ActionContext<Outcome>| cx.input))
/// 	.id();
/// world.spawn(RunNext::new(next));
/// ```
#[derive(Debug, Clone, Component, Reflect)]
#[require(RunNextAction<T>)]
#[reflect(Component)]
pub struct RunNext<T = Outcome>
where
	T: 'static
		+ Send
		+ Sync
		+ Clone
		+ PartialEq
		+ Reflect
		+ FromReflect
		+ bevy::reflect::Typed,
{
	/// The next action entity to call.
	pub target: Entity,
	/// If set, only jump when the input equals this value.
	pub if_input_matches: Option<T>,
}

impl RunNext<Outcome> {
	/// Always jump to `target`, threading an [`Outcome`].
	pub fn new(target: Entity) -> Self { Self::typed(target) }
	/// Only jump when the input is [`Outcome::PASS`].
	pub fn if_success(target: Entity) -> Self {
		Self::if_input(target, Outcome::PASS)
	}
	/// Only jump when the input is [`Outcome::FAIL`].
	pub fn if_failure(target: Entity) -> Self {
		Self::if_input(target, Outcome::FAIL)
	}
}

impl<T> RunNext<T>
where
	T: 'static
		+ Send
		+ Sync
		+ Clone
		+ PartialEq
		+ Reflect
		+ FromReflect
		+ bevy::reflect::Typed,
{
	/// Always jump to `target`, threading a `T`.
	pub fn typed(target: Entity) -> Self {
		Self {
			target,
			if_input_matches: None,
		}
	}
	/// Only jump when the input equals `value`.
	pub fn if_input(target: Entity, value: T) -> Self {
		Self {
			target,
			if_input_matches: Some(value),
		}
	}
}

/// Calls [`RunNext::target`] when the predicate matches, threading the input
/// value and returning the target's result; otherwise returns the input.
///
/// ## Errors
/// Errors if the caller has no [`RunNext`] component.
#[action(default)]
#[derive(Component)]
pub async fn RunNextAction<T>(cx: ActionContext<T>) -> Result<T>
where
	T: 'static
		+ Send
		+ Sync
		+ Clone
		+ PartialEq
		+ Reflect
		+ FromReflect
		+ bevy::reflect::Typed,
{
	let run_next = cx.caller.get_cloned::<RunNext<T>>().await?;
	if let Some(expected) = &run_next.if_input_matches {
		if &cx.input != expected {
			return cx.input.xok();
		}
	}
	cx.world()
		.entity(run_next.target)
		.call::<T, T>(cx.input)
		.await
}

#[cfg(test)]
mod tests {
	use super::*;

	fn passthrough() -> Action<Outcome, Outcome> {
		Action::new_pure(|cx: ActionContext<Outcome>| cx.input)
	}

	#[beet_core::test]
	async fn jumps_to_target() {
		let mut world = AsyncPlugin::world();
		let next = world
			.spawn(Action::<Outcome, Outcome>::new_fixed(Outcome::FAIL))
			.id();
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
		let next = world
			.spawn(Action::<Outcome, Outcome>::new_fixed(Outcome::FAIL))
			.id();
		world
			.spawn(RunNext::if_success(next))
			.call::<Outcome, Outcome>(Outcome::FAIL)
			.await
			.unwrap()
			.xpect_eq(Outcome::FAIL);
	}

	#[beet_core::test]
	async fn chains_through_states() {
		let mut world = AsyncPlugin::world();
		let state2 = world.spawn(passthrough()).id();
		let transition = world.spawn(RunNext::new(state2)).id();
		world
			.spawn(RunNext::new(transition))
			.call::<Outcome, Outcome>(Outcome::PASS)
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
}
