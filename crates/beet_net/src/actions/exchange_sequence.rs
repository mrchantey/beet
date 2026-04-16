use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// A Request/Response tool that will run each child until a
/// Fail<Outcome::Response> is reached, or else returns an Ok.
/// Errors are converted to a response.
pub fn exchange_sequence() -> impl Bundle {
	let fallback = SequenceAction::<Request, Response>::default().into_action();
	(
		ExcludeErrors(ChildError::NO_ACTION | ChildError::ACTION_MISMATCH),
		Action::<Request, Response>::new_async(
			async move |cx: ActionContext<Request>| -> Response {
				match cx.caller.call_detached(fallback, cx.input).await {
					Ok(Pass(_req)) => Response::ok(),
					// no child matched — return a simple plaintext not-found
					Ok(Fail(res)) => res,
					Err(err) => err.into_response(),
				}
			},
		),
	)
}
