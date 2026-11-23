//! The `POST /analytics` web client beacon route.
use super::router_analytics_ext;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A `POST /analytics` route accepting the web client beacon.
///
/// The client (the `<Analytics/>` script) posts page views (on load, a
/// heartbeat that slows with the page's age, and `pagehide`) plus click / scroll
/// / error events. Every post of a page view carries its `page_view_id`, and
/// compaction keeps the newest, so the final duration lands even when the server
/// never sees the (cached) page load itself.
pub(crate) fn analytics_handler() -> impl Bundle {
	(
		route::exchange("analytics", AnalyticsHandler),
		HttpMethod::Post,
	)
}

/// Parses the beacon into an [`AnalyticsEvent`], stamping the session (cookie),
/// the [`ClientOrigin`]'s country, and the raw ip only when the
/// router's [`AnalyticsConfig`] opts in, then triggers it for the analytics
/// observer to persist.
#[action]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
async fn AnalyticsHandler(cx: ActionContext<Request>) -> Result<Response> {
	let caller_id = cx.id();
	let world = cx.world();
	let request = cx.take();

	// session + origin from the beacon request headers (before consuming the body).
	let session = analytics_ext::session_from_cookies(request.headers());
	let origin = ClientOrigin::from_headers(request.headers());
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
		.then_some(origin.ip)
		.flatten()
		.map(|ip| SmolStr::from(ip.to_string()));
	let country =
		router_analytics_ext::origin_country(&world, caller_id, origin).await;

	// the beacon posts a json body; `into_value` parses it into a `Value` map.
	// A `sendBeacon` implementation may still declare `text/plain`, so a string
	// body is re-parsed as json rather than trusting the declared type.
	//
	// Every failure below is malformed client input, so it answers 400. This is a
	// public endpoint that bots probe constantly: a 500 would file their junk as a
	// fault of ours, logging an `Internal Error` per probe.
	let body = request
		.into_value()
		.await
		.map_err(|err| HttpError::bad_request(format!("beacon body: {err}")))?;
	let body = match body {
		Value::Str(text) => MediaType::Json
			.deserialize::<Value>(text.as_bytes())
			.map_err(|err| {
				HttpError::bad_request(format!("beacon body: {err}"))
			})?,
		body => body,
	};
	let event = AnalyticsEvent::from_beacon(body, session, ip, country)
		.map_err(|err| {
			HttpError::bad_request(format!("beacon event: {err}"))
		})?;
	world
		.with(move |world: &mut World| world.trigger(event))
		.await;

	Ok(Response::ok())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	const PAYLOAD: &str = r#"{"page_view_id":"0192f8a0-0000-7000-8000-000000000001","path":"/docs","duration_ms":1200}"#;

	/// The client ip the [`GeoIp`] stand-in resolves to `NZ`.
	const NZ_IP: &str = "203.0.113.7";

	/// Exchange a beacon `request` against a default router carrying a [`GeoIp`]
	/// stand-in, asserting the body actually parsed (the `/docs` path comes from
	/// it, not a default) and returning every event observed.
	async fn beacon_events(request: Request) -> Vec<AnalyticsEvent> {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::<
			AnalyticsEvent,
		>::new()));
		let recorder = captured.clone();
		world.add_observer(move |ev: On<AnalyticsEvent>| {
			ev.event().event_kind.xpect_eq(AnalyticsEventKind::PageView);
			ev.event().path.as_str().xpect_eq("/docs");
			recorder.lock().unwrap().push(ev.event().clone());
		});
		// `Router::with_defaults` already wires `analytics_handler()` under json + std.
		let root = world
			.spawn((
				Router::with_defaults(),
				GeoIp::from_table([(NZ_IP.parse().unwrap(), "NZ")]),
			))
			.flush();
		world
			.entity_mut(root)
			.exchange(request)
			.await
			.status()
			.xpect_eq(StatusCode::OK);
		captured.lock().unwrap().clone()
	}

	#[beet_core::test]
	async fn accepts_post_and_triggers_page_view() {
		beacon_events(Request::with_json_str("analytics", PAYLOAD))
			.await
			.len()
			.xpect_eq(1);
	}

	/// A `sendBeacon` may post the json body declared as `text/plain`; the
	/// handler re-parses a string body (guarding the regression where every web
	/// page view collapsed to a phantom `/`).
	#[beet_core::test]
	async fn accepts_text_plain_beacon() {
		beacon_events(Request::with_media(
			"analytics",
			MediaBytes::new_text(PAYLOAD),
		))
		.await
		.len()
		.xpect_eq(1);
	}

	/// The beacon's country is its origin's: the proxy's header when sent, else
	/// the database's lookup of the client ip.
	#[beet_core::test]
	async fn records_the_origin_country() {
		let request = || {
			Request::with_json_str("analytics", PAYLOAD)
				.with_header_raw("cf-connecting-ip", NZ_IP)
		};
		beacon_events(request().with_header_raw("cf-ipcountry", "AU")).await[0]
			.country
			.as_deref()
			.xpect_eq(Some("AU"));
		beacon_events(request()).await[0]
			.country
			.as_deref()
			.xpect_eq(Some("NZ"));
	}

	/// A junk beacon body is the client's fault, so it answers 400.
	///
	/// Regression guard: an unparseable body raised an opaque error, so the
	/// public beacon endpoint answered `500` and logged an `Internal Error` for
	/// every bot that probed it.
	#[beet_core::test]
	async fn malformed_body_is_a_bad_request() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world.spawn(Router::with_defaults()).flush();
		for body in ["", "not-json", "[1,2,3]"] {
			world
				.entity_mut(root)
				.exchange(Request::with_json_str("analytics", body))
				.await
				.status()
				.xpect_eq(StatusCode::BAD_REQUEST);
		}
	}
}
