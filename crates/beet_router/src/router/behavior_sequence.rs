use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Marker that makes a route run its children as a behaviour-tree sequence: each
/// child `Action<(), Outcome>` runs in order, the route returning [`Response::ok`]
/// if every step passes, an error response if a step errors (eg a [`Command`]
/// whose process exits non-zero), or `500` if a step returns [`Fail`].
///
/// The behaviour-tree analogue of [`ExchangeSequence`](crate::prelude::ExchangeSequence):
/// where that threads the request through `Action<Request, Response>` steps, this
/// runs `()`-input steps that don't read the request, eg shelling out with
/// [`Command`]. A child without a matching `Action<(), Outcome>` is skipped via the
/// required [`ExcludeErrors`].
///
/// ```bsx
/// <Route path="build" {BehaviorSequence}>
///   <Command exe="cargo" args={["build", "--release"]}/>
/// </Route>
/// ```
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(
	ExcludeErrors = ExcludeErrors(ChildError::NO_ACTION | ChildError::ACTION_MISMATCH),
	Action<Request, Response> = behavior_sequence_action(),
)]
pub struct BehaviorSequence;

/// The `Action<Request, Response>` behind [`BehaviorSequence`]: runs the entity's
/// children as a behaviour-tree [`SequenceAction`] (input `()`), mapping its
/// outcome — or a step error — onto a response.
fn behavior_sequence_action() -> Action<Request, Response> {
	let sequence = SequenceAction::<(), ()>::default().into_action();
	Action::<Request, Response>::new_async(
		async move |cx: ActionContext<Request>| -> Response {
			match cx.caller.call_detached(sequence, ()).await {
				// every step passed
				Ok(Pass(())) => Response::ok(),
				// a step returned `Fail` (control-flow failure, not an error)
				Ok(Fail(())) => {
					Response::from_status(StatusCode::INTERNAL_SERVER_ERROR)
				}
				// a step errored (eg a `Command` process exited non-zero)
				Err(err) => err.into_response(),
			}
		},
	)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn all_pass() {
		AsyncPlugin::world()
			.spawn((BehaviorSequence, children![
				Action::<(), Outcome>::new_pure(|_: ActionContext<()>| Pass(())),
				Action::<(), Outcome>::new_pure(|_: ActionContext<()>| Pass(())),
			]))
			.call::<Request, Response>(Request::get("test"))
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn step_error_becomes_error_response() {
		AsyncPlugin::world()
			.spawn((BehaviorSequence, children![
				Action::<(), Outcome>::new_async(async |_: ActionContext<()>| {
					Err(bevyhow!("boom"))?
				}),
			]))
			.call::<Request, Response>(Request::get("test"))
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
	}
}
