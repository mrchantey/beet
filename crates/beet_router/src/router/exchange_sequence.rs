use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Exchange control-flow that runs each child sequentially.
/// Returns [`Response::ok`] if all children pass, or the first
/// [`Fail`] response. Errors are converted to a response.
pub fn exchange_sequence() -> impl Bundle {
	let sequence = SequenceAction::<Request, Response>::default().into_action();
	let action = Action::<Request, Response>::new_async(
		async move |cx: ActionContext<Request>| -> Response {
			match cx.caller.call_detached(sequence, cx.input).await {
				Ok(Pass(_req)) => Response::ok(),
				// child returned Fail — use that response
				Ok(Fail(res)) => res,
				Err(err) => err.into_response(),
			}
		},
	);
	(
		ExcludeErrors(ChildError::NO_ACTION | ChildError::ACTION_MISMATCH),
		ExchangeAction::from_action(action.clone()),
		action,
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
			.spawn((exchange_sequence(), children![
				Action::<Request, Outcome<Request, Response>>::new_pure(
					|cx: ActionContext<Request>| Pass(cx.input),
				),
				Action::<Request, Outcome<Request, Response>>::new_pure(
					|cx: ActionContext<Request>| Pass(cx.input),
				),
			]))
			.exchange(Request::get("test"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn first_fail_stops() {
		AsyncPlugin::world()
			.spawn((exchange_sequence(), children![
				Action::<Request, Outcome<Request, Response>>::new_pure(
					|_cx: ActionContext<Request>| {
						Fail(Response::from_status(StatusCode::IM_A_TEAPOT))
					},
				),
				Action::<Request, Outcome<Request, Response>>::new_pure(
					|cx: ActionContext<Request>| Pass(cx.input),
				),
			]))
			.exchange(Request::get("test"))
			.await
			.status()
			.xpect_eq(StatusCode::IM_A_TEAPOT);
	}

	#[beet_core::test]
	async fn works_with_router() {
		(AsyncPlugin, RouterPlugin)
			.into_world()
			.spawn((router(), children![route(
				"seq",
				(exchange_sequence(), children![
					Action::<Request, Outcome<Request, Response>>::new_pure(
						|cx: ActionContext<Request>| Pass(cx.input),
					),
				]),
			)]))
			.exchange(Request::get("seq"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}
}
