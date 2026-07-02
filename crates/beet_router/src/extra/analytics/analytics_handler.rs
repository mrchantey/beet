//! The `POST /analytics` web client beacon route.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A `POST /analytics` route accepting the web client beacon.
///
/// The client (the `<Analytics/>` script) posts page views (on load, a 10s
/// heartbeat, and `pagehide`) plus click / scroll / error events. A page view's
/// `page_view_id` overwrites its stored row, so the final duration lands even
/// when the server never sees the (cached) page load itself.
pub fn analytics_handler() -> impl Bundle {
	(
		exchange_route("analytics", AnalyticsHandler),
		HttpMethod::Post,
	)
}

/// Parses the beacon into an [`AnalyticsEvent`], stamping the session (cookie),
/// a geoip country (from the client address), and the raw ip only when the
/// router's [`AnalyticsConfig`] opts in, then triggers it for the analytics
/// observer to persist.
#[action(handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
async fn AnalyticsHandler(cx: ActionContext<Request>) -> Result<Response> {
	let caller_id = cx.id();
	let world = cx.world();
	let request = cx.take();

	// session + address from the beacon request headers (before consuming the body).
	let session = analytics_ext::session_from_cookies(request.headers());
	let ip = analytics_ext::client_ip(request.headers());
	let country = ip
		.zip(
			world
				.with(|world: &mut World| world.get_resource::<GeoIp>().cloned())
				.await,
		)
		.and_then(|(ip, geoip)| geoip.country(ip));
	// storing the raw ip honors the same opt-in as the request stream; off by
	// default, so only a country is derived.
	let store_ip = world
		.with_state::<AncestorQuery<&AnalyticsConfig>, bool>(move |query| {
			query
				.get(caller_id)
				.map(|config| config.store_ip)
				.unwrap_or(false)
		})
		.await;
	let ip = store_ip
		.then(|| ip.map(|ip| SmolStr::from(ip.to_string())))
		.flatten();

	let body = request.into_value().await?;
	let event = AnalyticsEvent::from_beacon(body, session, ip, country)?;
	world.with(move |world: &mut World| world.trigger(event)).await;

	Ok(Response::ok())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[derive(Resource, Default)]
	struct AnalyticsHits(u32);

	#[beet_core::test]
	async fn accepts_post_and_triggers_page_view() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.init_resource::<AnalyticsHits>();
		world.add_observer(
			|ev: On<AnalyticsEvent>, mut hits: ResMut<AnalyticsHits>| {
				// the body must actually be parsed: the path comes from it, not a
				// default (guarding the json-body deserialize).
				ev.event().event_kind.xpect_eq(AnalyticsEventKind::PageView);
				ev.event().path.as_str().xpect_eq("/docs");
				hits.0 += 1;
			},
		);
		// `default_router` already wires `analytics_handler()` under json + std.
		let root = world.spawn(default_router()).flush();
		let payload = r#"{"page_view_id":"0192f8a0-0000-7000-8000-000000000001","path":"/docs","duration_ms":1200}"#;

		world
			.entity_mut(root)
			.exchange(Request::with_json_str("analytics", payload))
			.await
			.status()
			.xpect_eq(StatusCode::OK);

		world.resource::<AnalyticsHits>().0.xpect_eq(1);
	}
}
