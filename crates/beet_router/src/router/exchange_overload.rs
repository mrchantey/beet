//! The typed-route adapter layer: an [`ActionOverload<Request, Response>`] that
//! bridges a typed handler into the router's request/response dispatch.
//!
//! A route handler is a typed `Action<In, Out>` where `In: FromRequest` and
//! `Out: IntoResponseWithRequestParts`. Dispatch speaks `Request -> Response`, so
//! [`route::exchange_overload`] builds the adapter serving that pair: it extracts the input
//! from the request, calls the entity's canonical typed action, and converts the
//! output into a response. A handler that is already `Request -> Response` is its
//! own route action, and resolution prefers the canonical action, so its
//! coinciding overload is simply never invoked. [`IntoResponseWithRequestParts`] is the
//! output half of the conversion. The macro, [`route::exchange`], and
//! [`ExchangeScript`] all build the adapter through this one layer.
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
#[cfg(feature = "serde")]
use serde::Serialize;

/// The route dispatch overload: an [`ActionOverload`] serving
/// `Request -> Response` on top of a typed handler's canonical action.
pub type ExchangeOverload = ActionOverload<Request, Response>;

/// Trait for converting a typed handler's output into a [`Response`], the output
/// half of the [`ExchangeOverload`] conversion.
///
/// Blanket impls cover the main cases:
/// - Types implementing [`Serialize`] (serde content negotiation)
///
/// Concrete impls exist for [`MediaBytes`] and [`PageRequest`] (a render root).
pub trait IntoResponseWithRequestParts<M>
where
	Self: Sized,
{
	/// Converts this output value into a response.
	fn into_response_with_request_parts(
		self,
		caller: AsyncEntity,
		parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>>;
}

/// Concrete impl for [`Response`] — an action that already produced a
/// fully-formed response (eg a redirect) passes it through unchanged.
impl IntoResponseWithRequestParts<Self> for Response {
	fn into_response_with_request_parts(
		self,
		_caller: AsyncEntity,
		_parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>> {
		Box::pin(async move { Ok(self) })
	}
}

/// Concrete impl for [`MediaBytes`] — wraps the bytes directly
/// into a response without content negotiation.
impl IntoResponseWithRequestParts<Self> for MediaBytes {
	fn into_response_with_request_parts(
		self,
		_caller: AsyncEntity,
		_parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>> {
		Box::pin(async move { Response::ok().with_media(self).xok() })
	}
}

/// Marker type for the [`Serialize`] blanket impl of [`IntoResponseWithRequestParts`].
#[cfg(feature = "serde")]
#[derive(TypePath)]
pub struct SerdeIntoResponseMarker;
#[cfg(feature = "serde")]
impl<T> IntoResponseWithRequestParts<SerdeIntoResponseMarker> for T
where
	T: 'static + Send + Sync + Serialize,
{
	fn into_response_with_request_parts(
		self,
		_caller: AsyncEntity,
		parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>> {
		Box::pin(async move {
			let accept = match parts.headers.get_or_default::<header::Accept>()
			{
				Ok(accept) => accept,
				Err(err) => {
					return HttpError::new(
						StatusCode::BAD_REQUEST,
						format!("failed to parse accept headers: {}", err),
					)
					.into_response()
					.xok();
				}
			};
			let bytes = MediaType::serialize_accepts(&accept, &self)?;
			Response::ok().with_media(bytes).xok()
		})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// An `In = Request` route handler is its own route action, so its overload
	/// pair coincides with its canonical one. Resolution prefers the canonical
	/// action, so the overload is never invoked and cannot recurse.
	#[beet_core::test]
	async fn request_route_dispatches_through_its_canonical_action() {
		(AsyncPlugin, RouterPlugin)
			.into_world()
			.spawn((Router::with_defaults(), children![route::exchange(
				"echo",
				exchange_ext::handler(|req| {
					Response::ok().with_body(req.take().path_string())
				})
			)]))
			.exchange(Request::get("echo"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("echo");
	}

	/// Shouts the request path back, a typed `RequestParts -> MediaBytes` handler.
	#[action(handler_only)]
	#[derive(Default, Clone, Component, Reflect)]
	#[reflect(Component)]
	async fn Shout(cx: ActionContext<RequestParts>) -> MediaBytes {
		MediaBytes::new_text(cx.take().path_string().to_uppercase())
	}

	/// A typed handler dispatches through its overload: the request becomes the
	/// typed input and the typed output becomes the response.
	#[beet_core::test]
	async fn typed_route_dispatches_through_its_overload() {
		(AsyncPlugin, RouterPlugin)
			.into_world()
			.spawn((Router::with_defaults(), children![route::exchange(
				"shout", Shout
			)]))
			.exchange(Request::get("shout"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("SHOUT");
	}
}
