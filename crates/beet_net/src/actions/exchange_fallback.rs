use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// A Request/Response tool that will try each child until an
/// Outcome::Response is reached, or else returns a NotFound.
/// Errors are converted to a response.
pub fn exchange_fallback() -> impl Bundle {
	let fallback = FallbackAction::<Request, Response>::default().into_action();
	(
		ExcludeErrors(ChildError::NO_ACTION | ChildError::ACTION_MISMATCH),
		Action::<Request, Response>::new_async(
			async move |cx: ActionContext<Request>| -> Response {
				match cx.caller.call_detached(fallback, cx.input).await {
					Ok(Pass(res)) => res,
					// no child matched — return a simple plaintext not-found
					Ok(Fail(req)) => {
						let text = format!(
							"Resource not found: {}",
							req.path_string()
						);
						let accepts = req
							.headers()
							.get_or_default::<header::Accept>()
							.unwrap_or_default();
						let body = MediaType::serialize_accepts(
							&accepts,
							&format!(
								"Resource not found: {}",
								req.path_string()
							),
						)
						.unwrap_or_else(|_| MediaBytes::new_text(text));
						Response::from_status(StatusCode::NOT_FOUND)
							.with_media(body)
					}
					Err(err) => err.into_response(),
				}
			},
		),
	)
}
