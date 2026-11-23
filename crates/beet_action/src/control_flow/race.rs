use crate::prelude::*;
use beet_core::prelude::*;

/// Race control-flow component.
///
/// Runs all child actions concurrently with a clone of the same input,
/// resolving with the outcome of the first to finish, pass or fail, and
/// dropping the rest, which stops each at its next await (a dropped call
/// cancels its callee). A child erroring first is the race's error. With no
/// children it passes the input.
///
/// [`Parallel`] awaits every child instead.
///
/// Child error handling is controlled by [`BypassErrors`].
///
/// ## Errors
///
/// Errors depending on [`ChildError`] flags when a child has:
/// - no [`ActionMeta`]
/// - incompatible [`ActionMeta`] signature
#[action]
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn Race<Input = (), Output = ()>(
	cx: ActionContext<Input>,
) -> Result<Outcome<Input, Output>>
where
	Input: 'static + Send + Sync + Clone,
	Output: 'static + Send + Sync,
{
	let children =
		BehaviourChildren::valid_for::<Input, Outcome<Input, Output>>(
			&cx.world(),
			cx.id(),
			"race",
		)
		.await?;

	let world = cx.world().clone();
	let input = cx.input;

	let calls = children.into_iter().map(|child| {
		let world = world.clone();
		let input = input.clone();
		async move {
			world
				.entity(child)
				.call::<Input, Outcome<Input, Output>>(input)
				.await
		}
	});

	async_ext::race_all(calls)
		.await
		.unwrap_or(Ok(Outcome::Pass(input)))
}

impl Race {
	/// Create a default `Race<(), ()>`.
	pub fn new() -> Self { Self::default() }
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::actions::test_support;

	#[beet_core::test]
	async fn no_children() {
		AsyncPlugin::world()
			.spawn(Race::new())
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Pass(()));
	}

	/// The first child to finish decides, and a slower sibling is stopped
	/// before its side effect lands.
	#[beet_core::test]
	async fn first_to_finish_wins() {
		let mut world = AsyncPlugin::world();
		let slow = Store::<bool>::default();
		let race = world
			.spawn((Race::new(), children![
				test_support::sleep_then_set(30, slow.clone()),
				Action::<(), Outcome>::new_pure(|_: ActionContext| {
					Outcome::FAIL.xok()
				}),
			]))
			.id();
		world
			.entity_mut(race)
			.call::<(), Outcome<(), ()>>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::Fail(()));
		test_support::settle(&mut world, 80).await;
		slow.get().xpect_false();
	}
}
