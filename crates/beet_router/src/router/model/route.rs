//! The Rust route constructors, the twin of the `<Route>` template in markup.
//!
//! A namespace module rather than associated fns: a route is not a [`Router`],
//! it is a child of one, so these read best module-qualified —
//! `route::new("about", handler)`, `route::exchange("add", action)`.
//!
//! - [`new`]: a path plus any bundle, the simplest form.
//! - [`exchange`]: a path plus a *typed* handler, including the
//!   [`ExchangeOverload`] adapting it to `Request -> Response` dispatch.
//! - [`exchange_overload`]: that adapter alone, for a handler that mounts its
//!   own path (the `#[action(route)]` macro's require site).
//! - [`fallback`]: the not-found sibling a router keeps last.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Creates a route from a path and bundle, the simplest route constructor.
pub fn new<B: Bundle>(path: &str, bundle: B) -> (PathPartial, B) {
	(PathPartial::new(path), bundle)
}

/// Creates a route bundle from a path and a typed action, including the
/// [`ExchangeOverload`] adapting the action to request/response dispatch.
pub fn exchange<In, Out, M, M1, M2, B>(
	path: &str,
	action: B,
) -> (PathPartial, B, ExchangeOverload)
where
	In: 'static + Send + Sync + FromRequest<M1>,
	Out: 'static + Send + Sync + IntoResponseWithRequestParts<M2>,
	B: Bundle + IntoAction<M, In = In, Out = Out>,
{
	(
		PathPartial::new(path),
		action,
		exchange_overload::<In, Out, M1, M2>(),
	)
}

/// [`exchange`] pinned to the serde markers.
///
/// [`Value`] converts two ways — the serde blanket, and the script-answer impl a
/// scripted route selects — so a handler speaking it leaves `M1`/`M2` ambiguous.
/// This names the ordinary choice, content negotiation both ways, so such a
/// route reads no differently from any other.
pub fn exchange_serde<In, Out, M, B>(
	path: &str,
	action: B,
) -> (PathPartial, B, ExchangeOverload)
where
	In: 'static + Send + Sync + FromRequest<SerdeFromRequestMarker>,
	Out: 'static
		+ Send
		+ Sync
		+ IntoResponseWithRequestParts<SerdeIntoResponseMarker>,
	B: Bundle + IntoAction<M, In = In, Out = Out>,
{
	exchange(path, action)
}

/// Builds the [`ExchangeOverload`] adapting a typed handler `Action<In, Out>` to
/// request/response dispatch.
///
/// Called at the route's require site, where the typed `In`/`Out` and their
/// [`FromRequest`] / [`IntoResponseWithRequestParts`] markers are known. The adapter calls
/// the entity's canonical action directly rather than re-entering resolution, so
/// a `Request -> Response` handler (whose overload pair coincides with its
/// canonical one) can never recurse.
pub fn exchange_overload<In, Out, M1, M2>() -> ExchangeOverload
where
	In: 'static + Send + Sync + FromRequest<M1>,
	Out: 'static + Send + Sync + IntoResponseWithRequestParts<M2>,
{
	ActionOverload::new(Action::new_async(
		async |cx: ActionContext<Request>| -> Result<Response> {
			let parts = cx.input.parts().clone();
			let input = In::from_request(cx.input).await?;
			let handler = cx
				.caller
				.get(|action: &Action<In, Out>| action.clone())
				.await?;
			let output: Out = cx.caller.call_detached(handler, input).await?;
			output
				.into_response_with_request_parts(cx.caller, parts)
				.await
		},
	))
}

/// Exchange control-flow that tries each child until one passes.
/// Returns the first [`Pass`] response, or a 404 not-found response
/// if no child matches. Errors are converted to a response.
pub fn fallback() -> impl Bundle {
	let fallback = FallbackAction::<Request, Response>::default().into_action();
	let action = Action::<Request, Response>::new_async(
		async move |cx: ActionContext<Request>| -> Response {
			match cx.caller.call_detached(fallback, cx.input).await {
				Ok(Pass(res)) => res,
				// no child matched — return a plaintext not-found
				Ok(Fail(req)) => {
					let text =
						format!("Resource not found: {}", req.path_string());
					// negotiate the error body against `Accept` when serde is
					// available; otherwise fall back to plain text
					#[cfg(feature = "serde")]
					let body = {
						let accepts = req
							.headers()
							.get_or_default::<header::Accept>()
							.unwrap_or_default();
						MediaType::serialize_accepts(&accepts, &text)
							.unwrap_or_else(|_| MediaBytes::new_text(text))
					};
					#[cfg(not(feature = "serde"))]
					let body = MediaBytes::new_text(text);
					Response::from_status(StatusCode::NOT_FOUND)
						.with_media(body)
				}
				Err(err) => err.into_response(),
			}
		},
	);
	(
		BypassErrors(ChildError::NO_ACTION | ChildError::ACTION_MISMATCH),
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
			.spawn((route::fallback(), children![
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
			.call::<Request, Response>(Request::get("test"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_eq("first".to_string());
	}

	#[beet_core::test]
	async fn no_match_returns_not_found() {
		AsyncPlugin::world()
			.spawn((route::fallback(), children![Action::<
				Request,
				Outcome<Response, Request>,
			>::new_pure(
				|cx: ActionContext<Request>| Fail(cx.input),
			),]))
			.call::<Request, Response>(Request::get("test"))
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
	}

	#[beet_core::test]
	async fn works_with_router() {
		(AsyncPlugin, RouterPlugin)
			.into_world()
			.spawn((Router::with_defaults(), children![route::new(
				"fb",
				(route::fallback(), children![Action::<
					Request,
					Outcome<Response, Request>,
				>::new_pure(
					|_cx: ActionContext<Request>| {
						Pass(Response::ok().with_body("matched"))
					},
				),]),
			)]))
			.exchange(Request::get("fb"))
			.await
			.unwrap_str()
			.await
			.xpect_eq("matched".to_string());
	}
}
