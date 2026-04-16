//! Exchange handlers that produce [`Action<Request, Response>`] components.
//!
//! These convenience functions create actions for common request/response
//! patterns, wrapping the `beet_action` primitives with HTTP-friendly APIs.

use crate::prelude::*;
use beet_action::prelude::*;


/// Creates a synchronous [`Action<Request, Response>`] from a closure.
///
/// ## Example
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// let mut world = World::new();
/// let mut entity = world.spawn(exchange_handler(|request| {
///     request.take().mirror()
/// }));
/// ```
pub fn exchange_handler<F>(func: F) -> Action<Request, Response>
where
	F: 'static
		+ Send
		+ Sync
		+ Clone
		+ FnOnce(ActionContext<Request>) -> Response,
{
	Action::new_pure(move |cx: ActionContext<Request>| Ok(func(cx)))
}

/// Creates an async [`Action<Request, Response>`] from a closure.
///
/// ## Example
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// let mut world = World::new();
/// let mut entity = world.spawn(exchange_handler_async(|request| async move {
///     request.mirror_parts()
/// }));
/// ```
pub fn exchange_handler_async<F, Fut>(func: F) -> Action<Request, Response>
where
	F: 'static + Send + Sync + Clone + FnOnce(Request) -> Fut,
	Fut: 'static + Send + Future<Output = Response>,
{
	Action::new_async(move |cx: ActionContext<Request>| {
		let fut = func(cx.input);
		async move { Ok(fut.await) }
	})
}

/// Creates a mirror exchange action that echoes requests back as responses.
///
/// Useful for testing and debugging exchange infrastructure.
pub fn mirror_exchange() -> Action<Request, Response> {
	exchange_handler(|req| req.take().mirror())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn handler_sync_works() {
		AsyncPlugin::world()
			.spawn(exchange_handler(|req| req.mirror_parts()))
			.exchange(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn handler_sync_custom_response() {
		AsyncPlugin::world()
			.spawn(exchange_handler(|_| {
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
			.spawn(exchange_handler_async(
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
			.spawn(exchange_handler_async(|_| async move {
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
