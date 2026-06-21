//! The typed-route adapter layer: install an `Action<Request, Response>` that
//! bridges a typed handler into the router's request/response dispatch.
//!
//! A route handler is a typed `Action<In, Out>` where `In: FromRequest` and
//! `Out: ExchangeRouteOut`. The router dispatches through an entity's
//! `Action<Request, Response>` slot, which only speaks `Request -> Response`.
//! [`RouteExchange`] bridges the two: its on-add installs an
//! `Action<Request, Response>` that extracts the input from the request, self-calls
//! the entity's typed `Action<In, Out>`, and converts the output into a response.
//! A handler that is already `Request -> Response` is its own route action, so no
//! adapter is installed (it would collide on the slot). [`ExchangeRouteOut`] is the
//! output half of the conversion. The macro, [`exchange_route`], and
//! `TransformExchangeScript` all install the adapter through this one layer.
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use core::any::TypeId;
#[cfg(feature = "serde")]
use serde::Serialize;

/// Installs the `Action<Request, Response>` route adapter for a sibling typed
/// handler `Action<In, Out>` on the same entity.
///
/// Built (via [`new`](Self::new)) at the route's require site, where the typed
/// `In`/`Out` and their [`FromRequest`] / [`ExchangeRouteOut`] markers are known,
/// then its on-add moves the prebuilt adapter into the entity's
/// `Action<Request, Response>` slot. A `Request -> Response` handler IS the route
/// action, so [`new`](Self::new) stores [`None`] and no adapter is installed
/// (avoiding a same-type slot collision). Not `Reflect`: it is regenerated through
/// the route component's `#[require]` on scene load, like the action it wraps.
#[derive(Clone, Component)]
#[component(on_add = on_add)]
pub struct RouteExchange(Option<Action<Request, Response>>);

impl RouteExchange {
	/// Builds the adapter for a typed handler `Action<In, Out>`, or [`None`] when
	/// the handler is already `Request -> Response` (it is its own route action).
	///
	/// `M1`/`M2` are the [`FromRequest`] / [`ExchangeRouteOut`] markers, inferred at
	/// the call site from `In`/`Out`.
	pub fn new<In, Out, M1, M2>() -> Self
	where
		In: 'static + Send + Sync + FromRequest<M1>,
		Out: 'static + Send + Sync + ExchangeRouteOut<M2>,
	{
		// a `Request -> Response` handler is the route action itself; installing an
		// adapter would clobber its `Action<Request, Response>` slot.
		if TypeId::of::<In>() == TypeId::of::<Request>()
			&& TypeId::of::<Out>() == TypeId::of::<Response>()
		{
			return Self(None);
		}
		Self(Some(Action::new_async(
			async |cx: ActionContext<Request>| -> Result<Response> {
				let parts = cx.input.parts().clone();
				let input = In::from_request(cx.input).await?;
				let output: Out = cx.caller.call(input).await?;
				output.into_route_response(cx.caller, parts).await
			},
		)))
	}
}

/// Moves the prebuilt adapter into the entity's `Action<Request, Response>` slot.
/// A passthrough route ([`None`]) is its own route action, so nothing is installed.
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	if let Some(adapter) = world
		.entity(cx.entity)
		.get::<RouteExchange>()
		.and_then(|route| route.0.clone())
	else {
		return;
	};
	world.commands().entity(cx.entity).insert(adapter);
}

/// Trait for converting a typed handler's output into a [`Response`], the output
/// half of the [`RouteExchange`] conversion.
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

/// Creates a route bundle from a path and a typed action, including the
/// [`RouteExchange`] adapter that bridges the action to request/response dispatch.
pub fn exchange_route<In, Out, M, M1, M2, B>(
	path: &str,
	action: B,
) -> (PathPartial, B, RouteExchange)
where
	In: 'static + Send + Sync + FromRequest<M1>,
	Out: 'static + Send + Sync + ExchangeRouteOut<M2>,
	B: Bundle + IntoAction<M, In = In, Out = Out>,
{
	(
		PathPartial::new(path),
		action,
		RouteExchange::new::<In, Out, M1, M2>(),
	)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// An `In = Request` route handler is its own route action: [`RouteExchange`]
	/// installs no adapter (the signature-check passthrough), so the route dispatches
	/// directly through the handler's own `Action<Request, Response>` slot with no
	/// same-type collision.
	#[beet_core::test]
	async fn request_route_dispatches_without_adapter() {
		(AsyncPlugin, RouterPlugin)
			.into_world()
			.spawn((default_router(), children![exchange_route(
				"echo",
				exchange_handler(|req| {
					Response::ok().with_body(req.take().path_string())
				})
			)]))
			.exchange(Request::get("echo"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("echo");
	}
}
