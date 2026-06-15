use crate::prelude::*;
use beet_core::prelude::*;

/// The single batteries-included router builder, available on std and no_std.
///
/// Wires the [`Router`] dispatch action plus the standard middleware and the
/// default app-level routes around the provided `routes`:
/// - [`Router`] for route lookup and dispatch (always).
/// - [`RequestLogger`] middleware for per-request logging (no_std core, always).
/// - [`HelpHandler`] / [`NavigateHandler`] middleware for `--help` / `--navigate`
///   support (std-only: they render through the scene pipeline).
/// - an `/app-info` scene route (std-only) and a `POST /analytics` route
///   (`json` + std), both of which require a [`PackageConfig`] resource.
/// - a cached `GET /js/reactivity.js` route (std-only) serving the thin-client
///   reactivity runtime, the asset the reactive renderer's injected script loads.
///
/// On no_std the std-only children/middleware are omitted and the not-found
/// fallback is a plain-text route listing; add any extra `Request`/`Response`
/// [`Middleware`] to the spawned entity yourself if wanted.
pub fn default_router() -> impl Bundle {
	(
		Router,
		RequestLogger::default(),
		// std-only middleware: rendered through the scene pipeline.
		#[cfg(feature = "std")]
		HelpHandler::default(),
		#[cfg(feature = "std")]
		NavigateHandler::default(),
		// The default app routes, attached directly to this (path-less) router
		// entity via `OnSpawn::insert_child` so each route keeps its own path.
		// `insert_child` (unlike a shared `children!`) composes without bevy's
		// duplicate-`Children` error, so the std-only `app-info` and `json` + std
		// `analytics` routes can be cfg-gated tuple elements, simply absent on
		// no_std. `app_info`/`analytics` both need a `PackageConfig` resource.
		#[cfg(feature = "std")]
		OnSpawn::insert_child(app_info()),
		// the cached `/js/reactivity.js` runtime route, so a served or statically
		// exported reactive page's auto-injected runtime script resolves.
		#[cfg(feature = "std")]
		OnSpawn::insert_child(reactivity_js_route()),
		#[cfg(all(feature = "json", feature = "std"))]
		OnSpawn::insert_child(analytics_handler()),
		// the same-port `/__client_io` websocket-upgrade endpoint, so every HTTP
		// router exposes the live-reload/socket channel on its own port.
		#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
		OnSpawn::insert_child(client_io_route()),
	)
}


#[cfg(all(feature = "json", feature = "std"))]
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[action(handler_only)]
	#[derive(Default, Clone, Component, Reflect)]
	#[reflect(Component)]
	async fn Foobar(_cx: ActionContext<RequestParts>) -> MediaBytes {
		MediaBytes::new_text("foobar")
	}

	#[beet_core::test(timeout_ms = 10000)]
	async fn wires_default_routes() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(pkg_config!());
		let root = world
			.spawn((default_router(), children![exchange_route(
				"foobar", Foobar
			)]))
			.flush();

		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get("app-info"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("App Info");

		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get("foobar"))
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::OK);

		let payload = r#"{"event_type":"x","client_timestamp":0,"event_data":{},"session_data":{}}"#;
		world
			.entity_mut(root)
			.call::<Request, Response>(Request::with_json_str(
				"analytics",
				payload,
			))
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::OK);

		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get("bingbong"))
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
	}

	/// A `children![a, b]` group sits alongside `default_router` with both
	/// routes preserved at top level.
	#[beet_core::test(timeout_ms = 10000)]
	async fn wires_multi_route_group() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(pkg_config!());
		let root = world
			.spawn((default_router(), children![
				exchange_route("foo", Foobar),
				exchange_route("bar", Foobar),
			]))
			.flush();

		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get("foo"))
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::OK);

		world
			.entity_mut(root)
			.call::<Request, Response>(Request::get("bar"))
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::OK);
	}
}
