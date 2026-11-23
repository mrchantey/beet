//! Shared fixtures for the tests of stopping work.
use crate::prelude::*;
use beet_core::prelude::*;

/// An async action that sleeps for `millis`, then sets `store` and passes,
/// so a stopped call is one whose `store` stays unset.
pub(crate) fn sleep_then_set(
	millis: u64,
	store: Store<bool>,
) -> Action<(), Outcome> {
	Action::new_async(move |_: ActionContext| async move {
		time_ext::sleep_millis(millis).await;
		store.set(true);
		Outcome::PASS.xok::<BevyError>()
	})
}

/// Drive `world` for `millis`, long enough for any work not stopped to land.
pub(crate) async fn settle(world: &mut World, millis: u64) {
	world
		.run_async_then(move |_| time_ext::sleep_millis(millis))
		.await;
}
