//! Request dispatch through an entity's [`ExchangeAction`], plus [`ExchangeEnd`]
//! for observability.
//!
//! The entity's own [`Action<Request, Response>`] slot is the *exchangeable*
//! action a caller invokes with [`call`](beet_action::prelude::AsyncEntityActionExt::call)
//! (a server host fills it with an `ActionTrigger`). Dispatch is separate: an
//! [`ExchangeAction`] holds the request handler the higher layer (`beet_router`)
//! installs, and [`exchange`](ExchangeExt::exchange) dispatches it. This lets
//! `beet_net` dispatch a request without naming the router, and lets one host both
//! fan a boot out (its slot) and dispatch per-request (its [`ExchangeAction`]).
use super::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// An arbitrary `Request -> Response` handler that an
/// [`exchange`](ExchangeExt::exchange) call dispatches.
///
/// The raw dispatch hook: it holds a fully constructed `Action<Request, Response>`
/// and nothing more. `beet_router`'s `Router` installs one wrapping the route-tree
/// dispatch; a test installs one wrapping a bare handler. Held off the entity's
/// [`Action`] slot (which a server fills with an `ActionTrigger`), so a host can
/// both fan a boot out and dispatch per-request.
#[derive(Clone, Component)]
pub struct ExchangeAction(pub Action<Request, Response>);

impl ExchangeAction {
	/// Wraps an existing `Action<Request, Response>` as the dispatch hook.
	pub fn new(action: Action<Request, Response>) -> Self { Self(action) }

	/// Dispatches a request through this exchange action on the given entity.
	pub async fn call(
		&self,
		entity: AsyncEntity,
		request: Request,
	) -> Result<Response> {
		entity.call_detached(self.0.clone(), request).await
	}
}

impl IntoAction<Self> for ExchangeAction {
	type In = Request;
	type Out = Response;
	fn into_action(self) -> Action<Request, Response> { self.0 }
}

/// Extension trait for dispatching a request through an entity's
/// [`ExchangeAction`] from an owned [`EntityWorldMut`].
///
/// std-only: it drives the app to completion via `run_async_then`, which needs the
/// std [`AsyncRunner`]. no_std consumers use [`AsyncExchangeExt`] on an
/// [`AsyncEntity`] instead.
#[cfg(feature = "std")]
#[extend::ext(name = ExchangeExt)]
pub impl EntityWorldMut<'_> {
	/// Dispatch a request and await the response.
	///
	/// If dispatch fails, logs the error and returns [`Response::internal_error`].
	fn exchange(
		mut self,
		request: impl Into<Request>,
	) -> impl Future<Output = Response> {
		let request = request.into();
		async move {
			self.run_async_then(move |entity| exchange(entity, request))
				.await
		}
	}

	/// Dispatch a request and return the response body as a string. For tests.
	fn exchange_str(
		self,
		request: impl Into<Request>,
	) -> impl Future<Output = String> {
		let fut = self.exchange(request);
		async move { fut.await.unwrap_str().await }
	}
}

/// Extension trait for dispatching a request through an entity's
/// [`ExchangeAction`] from an [`AsyncEntity`] handle.
#[extend::ext(name = AsyncExchangeExt)]
pub impl AsyncEntity {
	/// Dispatch a request and await the response.
	///
	/// If dispatch fails, logs the error and returns [`Response::internal_error`].
	fn exchange(
		&self,
		request: impl Into<Request>,
	) -> impl MaybeSend + Future<Output = Response> {
		exchange(self.clone(), request.into())
	}

	/// Dispatch a request and return the response body as a string. For tests.
	fn exchange_str(
		&self,
		request: impl Into<Request>,
	) -> impl MaybeSend + Future<Output = String> {
		let fut = self.exchange(request);
		async move { fut.await.unwrap_str().await }
	}
}

/// Dispatch `request` through `entity`'s [`ExchangeAction`], then fire
/// [`ExchangeEnd`] so [`exchange_stats`] can log the request. The shared body of
/// both `exchange` extension traits.
async fn exchange(entity: AsyncEntity, request: Request) -> Response {
	let start_time = Instant::now();
	let method = *request.method();
	let path = request.path_string();
	let res = match entity.get_cloned::<ExchangeAction>().await {
		Ok(action) => entity
			.call_detached(action.0, request)
			.await
			.unwrap_or_else(|err| {
				error!("Exchange failed on {:?}: {}", entity.id(), err);
				Response::internal_error()
			}),
		Err(_) => {
			error!("No ExchangeAction on entity {:?}", entity.id());
			Response::internal_error()
		}
	};
	let status = res.status();
	entity
		.trigger(move |entity| ExchangeEnd {
			entity,
			method,
			path,
			start_time,
			status,
		})
		.await
		.ok();
	res
}

/// Event triggered when an exchange completes.
///
/// Carries the request method/path captured at dispatch plus the response status
/// and start time, so observers (eg [`exchange_stats`]) can log per-request info
/// without a [`RequestMeta`] component on the handler entity.
#[derive(Clone, EntityEvent)]
pub struct ExchangeEnd {
	/// The entity that dispatched this request.
	pub entity: Entity,
	/// The request method, captured at dispatch.
	pub method: HttpMethod,
	/// The request path, captured at dispatch.
	pub path: String,
	/// When the exchange started.
	pub start_time: Instant,
	/// The HTTP status code of the response.
	pub status: StatusCode,
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn missing_exchange_action_returns_error() {
		AsyncPlugin::world()
			.spawn_empty()
			.exchange(Request::get("foo"))
			.await
			.status()
			.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
	}

	#[beet_core::test]
	async fn works() {
		AsyncPlugin::world()
			.spawn(ExchangeAction(exchange_handler(|req| req.take().mirror())))
			.exchange(Request::get("foo"))
			.await
			.status()
			.is_ok()
			.xpect_true();
	}

	#[beet_core::test]
	async fn exchange_str_works() {
		AsyncPlugin::world()
			.spawn(ExchangeAction(exchange_handler(|_| {
				Response::ok().with_body("hello")
			})))
			.exchange_str(Request::get("foo"))
			.await
			.xpect_eq("hello".to_string());
	}
}
