//! Core exchange types for request/response handling via the tool pattern.
//!
//! This module provides [`ExchangeExt`] for ergonomic request/response
//! exchanges on entities that have a [`Tool<Request, Response>`] component,
//! and [`ExchangeEnd`] for observability.
use beet_core::prelude::*;
use beet_tool::prelude::*;

/// Extension trait for performing request/response exchanges on entities
/// with a [`Tool<Request, Response>`] component.
///
/// This is a thin convenience wrapper around the tool call pattern,
/// converting the `Result<Response>` into a `Response` by logging
/// errors and returning an internal error response on failure.
#[extend::ext(name=ExchangeExt)]
pub impl EntityWorldMut<'_> {
	/// Send a request and await the response.
	///
	/// If the tool call fails, logs the error and returns
	/// [`Response::internal_error`].
	fn exchange(
		mut self,
		request: impl Into<Request>,
	) -> impl Future<Output = Response> {
		let start_time = Instant::now();
		let request = request.into();
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
					.trigger_then(move |entity| ExchangeEnd {
						entity,
						start_time,
						status,
					})
					.await;
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
	/// If the tool call fails, logs the error and returns
	/// [`Response::internal_error`].
	fn exchange(
		&self,
		request: impl Into<Request>,
	) -> impl MaybeSend + Future<Output = Response> {
		let request = request.into();
		let fut = self.call::<Request, Response>(request);
		async move {
			match fut.await {
				Ok(res) => res,
				Err(err) => {
					error!("Exchange failed: {}", err);
					Response::internal_error()
				}
			}
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
/// Contains timing and status information for metrics and logging.
#[derive(Clone, EntityEvent)]
pub struct ExchangeEnd {
	/// The entity that handled this exchange.
	pub entity: Entity,
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
	async fn missing_tool_returns_error() {
		AsyncPlugin::world()
			.spawn_empty()
			.exchange(Request::get("foo"))
			.await
			.status()
			.xpect_eq(StatusCode::InternalError);
	}

	#[beet_core::test]
	async fn works() {
		AsyncPlugin::world()
			.spawn(handler_exchange(|req| req.mirror()))
			.exchange(Request::get("foo"))
			.await
			.status()
			.is_ok()
			.xpect_true();
	}

	#[beet_core::test]
	async fn exchange_str_works() {
		AsyncPlugin::world()
			.spawn(handler_exchange(|_| Response::ok().with_body("hello")))
			.exchange_str(Request::get("foo"))
			.await
			.xpect_eq("hello".to_string());
	}
}
