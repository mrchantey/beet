//! The opt-in that makes a behaviour's outcome its load's status.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Serves a `() -> Outcome` behaviour as the canonical `Request -> Response`
/// load, so its outcome is the process's status: [`Pass`] a `200`, [`Fail`] a
/// `500` and a nonzero exit code.
///
/// An [`Outcome`] is a branch, not an error: a `Fail` at a `<Sequence>` root
/// means a step did not pass, at a `<Repeat>` root that the loop ended. Which
/// of those is the process failing is the scene's to say, so the load alone
/// exits zero whenever the call resolves and this spread makes the outcome
/// count:
///
/// ```bsx
/// <Sequence {(CallOnReady, OutcomeStatus)}>
///     <CheckOne/>
///     <CheckTwo/>
/// </Sequence>
/// ```
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(ActionOverload<Request, Response> = outcome_overload())]
pub struct OutcomeStatus;

/// The overload serving the load from the entity's `() -> Outcome` action.
fn outcome_overload() -> ActionOverload<Request, Response> {
	ActionOverload::new(Action::new_async(
		async |cx: ActionContext<Request>| -> Result<Response> {
			let behaviour = cx
				.caller
				.get(|action: &Action<(), Outcome>| action.clone())
				.await?;
			match cx.caller.call_detached(behaviour, ()).await? {
				Pass(()) => Response::ok(),
				Fail(()) => Response::internal_error(),
			}
			.xok()
		},
	))
}

#[cfg(test)]
mod test {
	use super::*;

	fn behaviour(outcome: Outcome) -> Action<(), Outcome> {
		Action::new_pure(move |_: ActionContext| outcome.xok())
	}

	#[beet_core::test]
	async fn pass_is_ok() {
		AsyncPlugin::world()
			.spawn((OutcomeStatus, behaviour(Outcome::PASS)))
			.call::<Request, Response>(Request::get("/"))
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn fail_is_internal_error() {
		AsyncPlugin::world()
			.spawn((OutcomeStatus, behaviour(Outcome::FAIL)))
			.call::<Request, Response>(Request::get("/"))
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
	}
}
