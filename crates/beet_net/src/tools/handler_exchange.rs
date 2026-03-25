//! Exchange handlers that produce [`Tool<Request, Response>`] components.
//!
//! These convenience functions create tools for common request/response
//! patterns, wrapping the `beet_tool` primitives with HTTP-friendly APIs.

use crate::prelude::*;
use beet_tool::prelude::*;

/// Creates a synchronous [`Tool<Request, Response>`] from a closure.
///
/// ## Example
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// let mut world = World::new();
/// let mut entity = world.spawn(handler_exchange(|request| {
///     request.take().mirror()
/// }));
/// ```
pub fn handler_exchange<F>(func: F) -> Tool<Request, Response>
where
	F: 'static + Send + Sync + Clone + FnOnce(FuncToolIn<Request>) -> Response,
{
	func_tool(move |input: FuncToolIn<Request>| Ok(func(input)))
}

/// Creates an async [`Tool<Request, Response>`] from a closure.
///
/// ## Example
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// let mut world = World::new();
/// let mut entity = world.spawn(handler_exchange_async(|request| async move {
///     request.mirror_parts()
/// }));
/// ```
pub fn handler_exchange_async<F, Fut>(func: F) -> Tool<Request, Response>
where
	F: 'static + Send + Sync + Clone + FnOnce(Request) -> Fut,
	Fut: 'static + Send + Future<Output = Response>,
{
	async_tool(move |input: AsyncToolIn<Request>| {
		let fut = func(input.input);
		async move { Ok(fut.await) }
	})
}

/// Creates a mirror exchange tool that echoes requests back as responses.
///
/// Useful for testing and debugging exchange infrastructure.
pub fn mirror_exchange() -> Tool<Request, Response> {
	handler_exchange(|req| req.take().mirror())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn handler_sync_works() {
		AsyncPlugin::world()
			.spawn(handler_exchange(|req| req.mirror_parts()))
			.exchange(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn handler_sync_custom_response() {
		AsyncPlugin::world()
			.spawn(handler_exchange(|_| {
				Response::from_status(StatusCode::IM_A_TEAPOT)
			}))
			.exchange(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::IM_A_TEAPOT);
	}

	#[beet_core::test]
	async fn handler_async_works() {
		AsyncPlugin::world()
			.spawn(handler_exchange_async(
				|req| async move { req.mirror_parts() },
			))
			.exchange(Request::get("/bar"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn handler_async_custom_response() {
		AsyncPlugin::world()
			.spawn(handler_exchange_async(|_| async move {
				Response::from_status(StatusCode::IM_A_TEAPOT)
			}))
			.exchange(Request::get("/bar"))
			.await
			.status()
			.xpect_eq(StatusCode::IM_A_TEAPOT);
	}

	#[beet_core::test]
	async fn mirror_works() {
		AsyncPlugin::world()
			.spawn(mirror_exchange())
			.exchange(Request::get("/mirror"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}
}
