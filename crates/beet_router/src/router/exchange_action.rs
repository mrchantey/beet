//! Type-erased action and response-conversion traits for route dispatch.
//!
//! [`ExchangeAction`] wraps a typed action into a uniform
//! `Action<Request, Response>` so the router can dispatch without
//! knowing concrete input/output types. [`ExchangeRouteOut`] converts
//! an action's output into a [`Response`].
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
#[cfg(feature = "serde")]
use serde::Serialize;

/// Type-erased `Action<Request, Response>` stored on each route entity.
/// Handles request extraction and response conversion so the router
/// can dispatch uniformly.
#[derive(Clone, Component)]
pub struct ExchangeAction {
	inner: Action<Request, Response>,
}

impl ExchangeAction {
	/// Creates an exchange action that calls the entity's own typed action,
	/// extracting input from the request and converting output to a response.
	pub fn new<In, Out, M1, M2>() -> Self
	where
		In: 'static + Send + Sync + FromRequest<M1>,
		Out: 'static + Send + Sync + ExchangeRouteOut<M2>,
	{
		Self {
			inner: Action::new_async(
				async |cx: ActionContext<Request>| -> Result<Response> {
					let parts = cx.input.parts().clone();
					let input = In::from_request(cx.input).await?;
					let output: Out = cx.caller.call(input).await?;
					output.into_route_response(cx.caller, parts).await
				},
			),
		}
	}

	/// Wraps an existing `Action<Request, Response>` directly.
	/// Use this when you already have a fully constructed action
	/// that handles its own request/response lifecycle.
	pub fn from_action(action: Action<Request, Response>) -> Self {
		Self { inner: action }
	}

	/// Creates an exchange action that calls a detached inner action
	/// instead of the entity's own action.
	pub fn new_detached<In, Out, Inner, M1, M2, M3>(inner: Inner) -> Self
	where
		In: 'static + Send + Sync + FromRequest<M1>,
		Out: 'static + Send + Sync + ExchangeRouteOut<M2>,
		Inner: 'static + Send + Sync + IntoAction<M3, In = In, Out = Out>,
	{
		let inner = inner.into_action();
		Self {
			inner: Action::new_async(
				async move |cx: ActionContext<Request>| -> Result<Response> {
					let parts = cx.input.parts().clone();
					let input = In::from_request(cx.input).await?;
					let output: Out =
						cx.caller.call_detached(inner, input).await?;
					output.into_route_response(cx.caller, parts).await
				},
			),
		}
	}

	/// Dispatches a request through this exchange action on the given entity.
	pub async fn call(
		&self,
		entity: AsyncEntity,
		request: Request,
	) -> Result<Response> {
		entity.call_detached(self.inner.clone(), request).await
	}
}

impl IntoAction<Self> for ExchangeAction {
	type In = Request;
	type Out = Response;
	fn into_action(self) -> Action<Request, Response> { self.inner }
}

/// Trait for converting an action's output into a [`Response`].
///
/// Blanket impls cover the main cases:
/// - Types implementing [`Serialize`] (serde content negotiation)
///
/// Concrete impls exist for [`MediaBytes`] and [`SceneEntity`].
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


/// Creates a route bundle from a path and action, including the
/// [`ExchangeAction`] for type-erased dispatch.
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
		ExchangeAction::new_detached(action),
	)
}
