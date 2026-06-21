//! Request dispatch through an entity's `Action<Request, Response>` slot, plus
//! [`EndExchange`] for observability.
//!
//! [`exchange`](ExchangeExt::exchange) is the dispatch verb: it calls the entity's
//! own `Action<Request, Response>` slot with the request, fires an [`EndExchange`]
//! for the stats observer, and maps a missing slot or handler error to a `500`.
//! The slot's handler is whatever the higher layer installed: `beet_router`'s
//! `Router` fills it with the route-tree dispatch, a test fills it with a bare
//! handler. This lets `beet_net` dispatch a request without naming the router.
use super::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Extension trait for dispatching a request through an entity's
/// `Action<Request, Response>` slot from an owned [`EntityWorldMut`].
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
/// `Action<Request, Response>` slot from an [`AsyncEntity`] handle.
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

/// Dispatch `request` through `entity`'s `Action<Request, Response>` slot, then
/// fire [`EndExchange`] so [`exchange_stats`] can log the request. The shared body
/// of both `exchange` extension traits. A missing slot or a handler error maps to
/// [`Response::internal_error`].
async fn exchange(entity: AsyncEntity, request: Request) -> Response {
	let start_time = Instant::now();
	let method = *request.method();
	let path = request.path_string();
	let res = match entity.get_cloned::<Action<Request, Response>>().await {
		Ok(action) => entity
			.call_detached(action, request)
			.await
			.unwrap_or_else(|err| {
				error!("Exchange failed on {:?}: {}", entity.id(), err);
				Response::internal_error()
			}),
		Err(_) => {
			error!(
				"No Action<Request, Response> on entity {:?}",
				entity.id()
			);
			Response::internal_error()
		}
	};
	let status = res.status();
	entity
		.trigger(move |entity| EndExchange {
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
pub struct EndExchange {
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
	async fn missing_action_returns_error() {
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
			.spawn(exchange_handler(|req| req.take().mirror()))
			.exchange(Request::get("foo"))
			.await
			.status()
			.is_ok()
			.xpect_true();
	}

	#[beet_core::test]
	async fn exchange_str_works() {
		AsyncPlugin::world()
			.spawn(exchange_handler(|_| Response::ok().with_body("hello")))
			.exchange_str(Request::get("foo"))
			.await
			.xpect_eq("hello".to_string());
	}
}
