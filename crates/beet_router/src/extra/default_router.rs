use crate::prelude::*;
use beet_core::prelude::*;

/// A batteries-included router bundle wiring the default app-level routes
/// (`/app-info`, `POST /analytics`) alongside the provided `routes`.
///
/// Requires a [`PackageConfig`] resource (eg via `pkg_config!()`). `routes` may
/// be a single route or a `children![..]` group; either is nested under a
/// path-less entity, so route paths are preserved.
pub fn default_router<B: Bundle>(routes: B) -> impl Bundle {
	(router(), children![app_info(), analytics_handler(), routes])
}


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
			.spawn(default_router(exchange_route("foobar", Foobar)))
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
				"analytics", payload,
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
}
