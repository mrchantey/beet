use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Exchange control-flow that tries each child until one passes.
/// Returns the first [`Pass`] response, or a 404 not-found response
/// if no child matches. Errors are converted to a response.
pub fn exchange_fallback() -> impl Bundle {
	let fallback = FallbackAction::<Request, Response>::default().into_action();
	let action = Action::<Request, Response>::new_async(
		async move |cx: ActionContext<Request>| -> Response {
			match cx.caller.call_detached(fallback, cx.input).await {
				Ok(Pass(res)) => res,
				// no child matched — return a plaintext not-found
				Ok(Fail(req)) => {
					let text =
						format!("Resource not found: {}", req.path_string());
					let accepts = req
						.headers()
						.get_or_default::<header::Accept>()
						.unwrap_or_default();
					let body = MediaType::serialize_accepts(
						&accepts,
						&format!("Resource not found: {}", req.path_string()),
					)
					.unwrap_or_else(|_| MediaBytes::new_text(text));
					Response::from_status(StatusCode::NOT_FOUND).with_media(body)
				}
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
	async fn first_pass_returns() {
		AsyncPlugin::world()
			.spawn((exchange_fallback(), children![
				Action::<Request, Outcome<Response, Request>>::new_pure(
					|_cx: ActionContext<Request>| {
						Pass(Response::ok().with_body("first"))
					},
				),
				Action::<Request, Outcome<Response, Request>>::new_pure(
					|_cx: ActionContext<Request>| {
						Pass(Response::ok().with_body("second"))
					},
				),
			]))
			.exchange(Request::get("test"))
			.await
			.unwrap_str()
			.await
			.xpect_eq("first".to_string());
	}

	#[beet_core::test]
	async fn no_match_returns_not_found() {
		AsyncPlugin::world()
			.spawn((exchange_fallback(), children![
				Action::<Request, Outcome<Response, Request>>::new_pure(
					|cx: ActionContext<Request>| Fail(cx.input),
				),
			]))
			.exchange(Request::get("test"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
	}

	#[beet_core::test]
	async fn works_with_router() {
		(AsyncPlugin, RouterPlugin)
			.into_world()
			.spawn((router(), children![route(
				"fb",
				(exchange_fallback(), children![
					Action::<Request, Outcome<Response, Request>>::new_pure(
						|_cx: ActionContext<Request>| {
							Pass(Response::ok().with_body("matched"))
						},
					),
				]),
			)]))
			.exchange(Request::get("fb"))
			.await
			.unwrap_str()
			.await
			.xpect_eq("matched".to_string());
	}
}
