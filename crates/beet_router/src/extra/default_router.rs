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
		// entity via `OnSpawn::insert_child` so each route keeps its own path. A
		// no-code BSX site requests the same set with `<.. DefaultAppRoutes>`.
		// `app_info`/`analytics` both need a `PackageConfig` resource.
		#[cfg(feature = "std")]
		OnSpawn::insert_child(app_info()),
		#[cfg(feature = "std")]
		OnSpawn::insert_child(reactivity_js_route()),
		#[cfg(all(feature = "json", feature = "std"))]
		OnSpawn::insert_child(analytics_handler()),
		#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
		OnSpawn::insert_child(client_io_route()),
	)
}

/// Markup-spawnable counterpart of the app routes [`default_router`] wires: an
/// entity carrying this receives the reactivity-runtime asset
/// (`/js/reactivity.js`), `/app-info`, `POST /analytics`, and the `/__client_io`
/// websocket channel as children, via [`spawn_default_app_routes`].
///
/// It exists so a no-code BSX site declares those routes from markup
/// (`<Router {(.., DefaultAppRoutes)}>`) without a Rust `default_router`. This is
/// the same reflect-component-plus-`On<Insert>`-observer shape as
/// [`RoutesDir`](crate::prelude::RoutesDir), the established way markup injects
/// routes. `app_info`/`analytics` need a [`PackageConfig`] resource.
#[cfg(feature = "std")]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct DefaultAppRoutes;

/// Observer that inserts the [`DefaultAppRoutes`] children (see its docs). Each
/// route is attached directly to the marked entity, keeping its own path.
#[cfg(feature = "std")]
pub fn spawn_default_app_routes(
	ev: On<Insert, DefaultAppRoutes>,
	mut commands: Commands,
) {
	let parent = ev.entity;
	commands.spawn((ChildOf(parent), app_info()));
	commands.spawn((ChildOf(parent), reactivity_js_route()));
	#[cfg(feature = "json")]
	commands.spawn((ChildOf(parent), analytics_handler()));
	#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
	commands.spawn((ChildOf(parent), client_io_route()));
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
