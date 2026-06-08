//! Core exchange types for request/response handling via the action pattern.
//!
//! This module provides [`ExchangeExt`] for ergonomic request/response
//! exchanges on entities that have an [`Action<Request, Response>`] component,
//! and [`ExchangeEnd`] for observability.
use super::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Extension trait for performing request/response exchanges on entities
/// with an [`Action<Request, Response>`] component.
///
/// This is a thin convenience wrapper around the action call pattern,
/// converting the `Result<Response>` into a `Response` by logging
/// errors and returning an internal error response on failure.
///
/// std-only: it drives the app to completion via `run_async_then`, which needs
/// the std [`AsyncRunner`]. no_std consumers use [`AsyncExchangeExt`] on an
/// [`AsyncEntity`] instead.
#[cfg(feature = "std")]
#[extend::ext(name=ExchangeExt)]
pub impl EntityWorldMut<'_> {
	/// Send a request and await the response.
	///
	/// If the action call fails, logs the error and returns
	/// [`Response::internal_error`].
	fn exchange(
		mut self,
		request: impl Into<Request>,
	) -> impl Future<Output = Response> {
		let start_time = Instant::now();
		let request = request.into();
		let method = *request.method();
		let path = request.path_string();
		async move {
			self.run_async_then(async move |entity| {
				let res = entity.call(request).await.unwrap_or_else(|err| {
					error!(
						"Exchange failed on entity {:?}: {}",
						entity.id(),
						err
					);
					Response::internal_error()
				});
				trace!(
					"Exchange on {:?} completed in {:?}",
					entity.id(),
					start_time.elapsed()
				);
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
			})
			.await
		}
	}

	/// Exchange a request and return the response body as a string.
	///
	/// Convenience method for testing and debugging.
	fn exchange_str(
		self,
		request: impl Into<Request>,
	) -> impl Future<Output = String> {
		let fut = self.exchange(request);
		async move { fut.await.unwrap_str().await }
	}
}

/// Extension trait for performing request/response exchanges on
/// [`AsyncEntity`] handles.
#[extend::ext(name=AsyncExchangeExt)]
pub impl AsyncEntity {
	/// Send a request and await the response.
	///
	/// If the action call fails, logs the error and returns
	/// [`Response::internal_error`].
	fn exchange(
		&self,
		request: impl Into<Request>,
	) -> impl MaybeSend + Future<Output = Response> {
		let start_time = Instant::now();
		let request = request.into();
		let method = *request.method();
		let path = request.path_string();
		let entity = self.clone();
		let fut = self.call::<Request, Response>(request);
		async move {
			let res = match fut.await {
				Ok(res) => res,
				Err(err) => {
					error!("Exchange failed: {}", err);
					Response::internal_error()
				}
			};
			// fire `ExchangeEnd` so `exchange_stats` can log method/path/status/
			// duration. This is the live-server path (mini/hyper backends call
			// through here), mirroring the std `ExchangeExt::exchange` variant.
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
	}

	/// Exchange a request and return the response body as a string.
	///
	/// Convenience method for testing and debugging.
	fn exchange_str(
		&self,
		request: impl Into<Request>,
	) -> impl MaybeSend + Future<Output = String> {
		let fut = self.exchange(request);
		async move { fut.await.unwrap_str().await }
	}
}

/// Event triggered when an exchange completes.
///
/// Carries the request method/path captured at dispatch plus the response
/// status and start time, so observers (eg [`exchange_stats`]) can log
/// per-request info without a [`RequestMeta`] component on the handler entity
/// (the live-server path dispatches the request through `call`, not as a
/// spawned component).
#[derive(Clone, EntityEvent)]
pub struct ExchangeEnd {
	/// The entity that handled this exchange.
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
