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
/// - a `GET /health` route (std-only) returning 200 + json metrics, the
///   load-balancer health check and autoscaling signal.
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
		// the default app routes, as children of this router entity (a no-code BSX
		// site gets the same set from the `<DefaultAppRoutes/>` template).
		#[cfg(feature = "std")]
		default_app_routes(),
	)
}

/// The default app routes as a bundle of [`OnSpawn::insert_child`] effects: the
/// reactivity-runtime asset (`/js/reactivity.js`), `/app-info`, `POST /analytics`,
/// and the `/__client_io` websocket channel, each attached as its own child so it
/// keeps its own path. Shared by [`default_router`] and the [`DefaultAppRoutes`]
/// template. `app_info`/`analytics` need a [`PackageConfig`] resource.
#[cfg(feature = "std")]
pub fn default_app_routes() -> impl Bundle {
	(
		OnSpawn::insert_child(app_info()),
		OnSpawn::insert_child(reactivity_js_route()),
		// the load-balancer health check + autoscaling signal.
		OnSpawn::insert_child(health_route()),
		#[cfg(feature = "json")]
		OnSpawn::insert_child(analytics_handler()),
		#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
		OnSpawn::insert_child(client_io_route()),
	)
}

/// Markup-spawnable [`default_app_routes`]: a template so a no-code BSX site
/// requests the default app routes with `<DefaultAppRoutes/>`, the same way it
/// places `<RouteSidebar/>` or `<RoutesDir/>`, without a Rust `default_router`.
/// Expanding at build leaves no marker behind, so a saved scene round-trips
/// cleanly.
#[cfg(feature = "std")]
#[template]
pub fn DefaultAppRoutes() -> impl Bundle { default_app_routes() }

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
			.exchange(Request::get("app-info"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("App Info");

		world
			.entity_mut(root)
			.exchange(Request::get("foobar"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);

		let payload = r#"{"event_type":"x","client_timestamp":0,"event_data":{},"session_data":{}}"#;
		world
			.entity_mut(root)
			.exchange(Request::with_json_str("analytics", payload))
			.await
			.status()
			.xpect_eq(StatusCode::OK);

		world
			.entity_mut(root)
			.exchange(Request::get("bingbong"))
			.await
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
			.exchange(Request::get("foo"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);

		world
			.entity_mut(root)
			.exchange(Request::get("bar"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}
}
