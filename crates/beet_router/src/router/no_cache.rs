use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Middleware that adds no-cache headers to every response, instructing
/// clients and proxies not to store the result.
///
/// Set `Cache-Control`, `Pragma` and `Expires` after the inner handler runs.
#[action]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = on_add_middleware::<Self, Request, Response>)]
pub async fn NoCacheHeaders(
	cx: ActionContext<(Request, Next<Request, Response>)>,
) -> Result<Response> {
	let (request, next) = cx.take();
	let mut response = next.call(request).await?;
	let headers = &mut response.parts.headers;
	headers.set::<header::CacheControl>(
		"no-cache, no-store, must-revalidate".to_string(),
	);
	headers.set::<header::Pragma>("no-cache".to_string());
	headers.set::<header::Expires>("0".to_string());
	Ok(response)
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	#[action(handler_only)]
	#[derive(Default, Clone, Component, Reflect)]
	#[reflect(Component)]
	async fn Hello(_cx: ActionContext<RequestParts>) -> MediaBytes {
		MediaBytes::new_text("Hello")
	}

	#[beet_core::test]
	async fn sets_no_cache_headers() {
		let response = router_world()
			.spawn((default_router(children![exchange_route("", Hello)]), NoCacheHeaders))
			.call::<Request, Response>(Request::get(""))
			.await
			.unwrap();

		response
			.headers
			.get::<header::CacheControl>()
			.unwrap()
			.unwrap()
			.xpect_eq("no-cache, no-store, must-revalidate");
		response
			.headers
			.get::<header::Pragma>()
			.unwrap()
			.unwrap()
			.xpect_eq("no-cache");
		response
			.headers
			.get::<header::Expires>()
			.unwrap()
			.unwrap()
			.xpect_eq("0");
	}
}
