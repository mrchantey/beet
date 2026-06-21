//! The typed-route transform: build an [`ExchangeAction`] from a typed handler.
//!
//! A route handler is a typed `Action<In, Out>` where `In: FromRequest` and
//! `Out: ExchangeRouteOut`. The router dispatches through the type-erased
//! [`ExchangeAction`] (defined in `beet_net`), which only speaks
//! `Request -> Response`. [`TransformExchange`] bridges the two: it wraps a typed
//! handler in an [`ExchangeAction`] that extracts the input from the request and
//! converts the output into a response. [`ExchangeRouteOut`] is the output half of
//! that conversion.
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
#[cfg(feature = "serde")]
use serde::Serialize;

/// Builds the type-erased [`ExchangeAction`] for a typed route handler.
///
/// A purely namespacing type: its [`new`](Self::new) /
/// [`new_detached`](Self::new_detached) constructors wrap a typed
/// `Action<In, Out>` (with `In: FromRequest`, `Out: ExchangeRouteOut`) in an
/// [`ExchangeAction`] that extracts the request input and converts the output to a
/// [`Response`].
pub enum TransformExchange {}

impl TransformExchange {
	/// Builds an [`ExchangeAction`] that calls the entity's own typed action,
	/// extracting input from the request and converting output to a response.
	pub fn new<In, Out, M1, M2>() -> ExchangeAction
	where
		In: 'static + Send + Sync + FromRequest<M1>,
		Out: 'static + Send + Sync + ExchangeRouteOut<M2>,
	{
		ExchangeAction::new(Action::new_async(
			async |cx: ActionContext<Request>| -> Result<Response> {
				let parts = cx.input.parts().clone();
				let input = In::from_request(cx.input).await?;
				let output: Out = cx.caller.call(input).await?;
				output.into_route_response(cx.caller, parts).await
			},
		))
	}

	/// Builds an [`ExchangeAction`] that calls a detached inner action
	/// instead of the entity's own action.
	pub fn new_detached<In, Out, Inner, M1, M2, M3>(
		inner: Inner,
	) -> ExchangeAction
	where
		In: 'static + Send + Sync + FromRequest<M1>,
		Out: 'static + Send + Sync + ExchangeRouteOut<M2>,
		Inner: 'static + Send + Sync + IntoAction<M3, In = In, Out = Out>,
	{
		let inner = inner.into_action();
		ExchangeAction::new(Action::new_async(
			async move |cx: ActionContext<Request>| -> Result<Response> {
				let parts = cx.input.parts().clone();
				let input = In::from_request(cx.input).await?;
				let output: Out = cx.caller.call_detached(inner, input).await?;
				output.into_route_response(cx.caller, parts).await
			},
		))
	}
}

/// Trait for converting a typed handler's output into a [`Response`], the output
/// half of the [`TransformExchange`] conversion.
///
/// Blanket impls cover the main cases:
/// - Types implementing [`Serialize`] (serde content negotiation)
///
/// Concrete impls exist for [`MediaBytes`] and [`PageRequest`] (a render root).
pub trait ExchangeRouteOut<M>
where
	Self: Sized,
{
	/// Converts this output value into a response.
	fn into_route_response(
		self,
		caller: AsyncEntity,
		parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>>;
}

/// Concrete impl for [`Response`] — an action that already produced a
/// fully-formed response (eg a redirect) passes it through unchanged.
impl ExchangeRouteOut<Self> for Response {
	fn into_route_response(
		self,
		_caller: AsyncEntity,
		_parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>> {
		Box::pin(async move { Ok(self) })
	}
}

/// Concrete impl for [`MediaBytes`] — wraps the bytes directly
/// into a response without content negotiation.
impl ExchangeRouteOut<Self> for MediaBytes {
	fn into_route_response(
		self,
		_caller: AsyncEntity,
		_parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>> {
		Box::pin(async move { Response::ok().with_media(self).xok() })
	}
}

/// Marker type for the [`Serialize`] blanket impl of [`ExchangeRouteOut`].
#[cfg(feature = "serde")]
#[derive(TypePath)]
pub struct SerdeIntoResponseMarker;
#[cfg(feature = "serde")]
impl<T> ExchangeRouteOut<SerdeIntoResponseMarker> for T
where
	T: 'static + Send + Sync + Serialize,
{
	fn into_route_response(
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

/// Creates a route from a path and bundle, the simplest route constructor.
pub fn route<B: Bundle>(path: &str, bundle: B) -> (PathPartial, B) {
	(PathPartial::new(path), bundle)
}

/// Creates a route bundle from a path and action, including the type-erased
/// [`ExchangeAction`] (built by [`TransformExchange::new_detached`]) for dispatch.
pub fn exchange_route<In, Out, M, M1, M2, B>(
	path: &str,
	action: B,
) -> (PathPartial, B, ExchangeAction)
where
	In: 'static + Send + Sync + FromRequest<M1>,
	Out: 'static + Send + Sync + ExchangeRouteOut<M2>,
	B: Bundle + Clone + IntoAction<M, In = In, Out = Out>,
{
	(
		PathPartial::new(path),
		action.clone(),
		TransformExchange::new_detached(action),
	)
}
