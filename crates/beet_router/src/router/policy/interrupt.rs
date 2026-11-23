//! Interrupt middleware for action subtrees.
//!
//! Interruption is opt-in via these middleware, each queueing an
//! [`InterruptRun`], which stops every action call in flight beneath the host,
//! async or parked. [`InterruptOnRun`] stops them before dispatching (a parent
//! re-run should not leave old children running), while [`InterruptOnEnd`]
//! stops any still running once the inner action resolves. Descendants
//! carrying [`NoInterrupt`] are skipped.

use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Middleware that interrupts running descendants before dispatching,
/// stopping the calls a previous run left in flight.
#[action]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn InterruptOnRun(
	cx: ActionContext<(Request, Next<Request, Response>)>,
) -> Result<Response> {
	let caller = cx.caller.clone();
	let (request, next) = cx.take();
	caller.queue(InterruptRun::new()).await??;
	next.call(request).await
}

/// Middleware that interrupts running descendants after the inner action
/// resolves, cancelling any children still running once the parent is done.
#[action]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn InterruptOnEnd(
	cx: ActionContext<(Request, Next<Request, Response>)>,
) -> Result<Response> {
	let caller = cx.caller.clone();
	let (request, next) = cx.take();
	let response = next.call(request).await;
	caller.queue(InterruptRun::new()).await??;
	response
}

#[cfg(test)]
mod test {
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// Inner action that ignores its request and returns an ok response.
	fn ok_inner() -> Action<Request, Response> {
		Action::new_async(|_cx: ActionContext<Request>| async {
			Ok(Response::ok())
		})
	}

	/// Spawns `wrapped` as the action on a parent with a child already in a
	/// [`Running`] state, returning the parent and child entities.
	fn spawn_with_running_child(
		world: &mut World,
		wrapped: Action<Request, Response>,
	) -> (Entity, Entity) {
		let child = world.spawn(ContinueRun::<(), Outcome>::default()).id();
		world
			.entity_mut(child)
			.call_with((), OutHandler::<Outcome>::new(|_, _| Ok(())))
			.unwrap();
		world.get::<Running<Outcome>>(child).xpect_some();
		let parent = world.spawn(wrapped).add_child(child).id();
		(parent, child)
	}

	#[beet_core::test]
	async fn interrupt_on_run_clears_descendants() {
		let mut world = AsyncPlugin::world();
		let (parent, child) = spawn_with_running_child(
			&mut world,
			super::InterruptOnRun.wrap(ok_inner()),
		);

		world
			.entity_mut(parent)
			.call::<Request, Response>(Request::get(""))
			.await
			.unwrap();

		world.get::<Running<Outcome>>(child).xpect_none();
	}

	#[beet_core::test]
	async fn interrupt_on_end_clears_descendants() {
		let mut world = AsyncPlugin::world();
		let (parent, child) = spawn_with_running_child(
			&mut world,
			super::InterruptOnEnd.wrap(ok_inner()),
		);

		world
			.entity_mut(parent)
			.call::<Request, Response>(Request::get(""))
			.await
			.unwrap();

		world.get::<Running<Outcome>>(child).xpect_none();
	}
}
