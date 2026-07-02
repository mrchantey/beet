//! The server-side request-analytics middleware.
//!
//! Records one [`AnalyticsKind::Request`] event per routed request across every
//! exchange-based transport (HTTP, CLI, socket). This is the raw traffic log,
//! the "what hit the server" stream; the page-view stream comes from the web
//! client beacon and the in-world [`Navigator`](crate::prelude::Navigator).
//!
//! Opt-in and gated: it records only when an ancestor carries an
//! [`AnalyticsConfig`] with `record_requests` set (the default). The `/analytics`
//! beacon and `/health` check are skipped, so the log never feedback-loops on
//! itself or fills with load-balancer pings.
use super::router_analytics_ext;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Paths whose first segment is skipped: the beacon endpoint (avoids a feedback
/// loop) and the health check (avoids load-balancer noise).
const SKIP_SEGMENTS: [&str; 2] = ["analytics", "health"];

/// Middleware that records a [`AnalyticsKind::Request`] event per routed request.
///
/// Spread on a router alongside an [`AnalyticsConfig`] to enable it (the
/// `<SiteAnalytics/>` template does both). Reads the client address, session
/// cookie, and user agent from the request, the status from the response, and a
/// geoip country from the address, then triggers an [`AnalyticsEvent`] the
/// analytics observer persists.
#[action]
#[derive(Default, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = on_add_middleware::<Self, Request, Response>)]
pub async fn AnalyticsMiddleware(
	cx: ActionContext<(Request, Next<Request, Response>)>,
) -> Result<Response> {
	let caller = cx.caller.clone();
	let caller_id = caller.id();
	let world = caller.world().clone();
	let (request, next) = cx.take();

	// the config gates recording; without it (or with requests disabled) this is a
	// plain pass-through with no per-request analytics work.
	let config = world
		.with_state::<AncestorQuery<&AnalyticsConfig>, Option<AnalyticsConfig>>(
			move |query| query.get(caller_id).ok().cloned(),
		)
		.await;
	let Some(config) = config.filter(|config| config.record_requests) else {
		return next.call(request).await;
	};

	// the `/analytics` beacon and `/health` check are not page traffic.
	if request
		.first_segment()
		.is_some_and(|segment| SKIP_SEGMENTS.contains(&segment))
	{
		return next.call(request).await;
	}

	// read everything needed before the body-consuming `next.call`.
	let path = request.path_string();
	let method = request.method().to_string();
	let ip = analytics_ext::client_ip(request.headers());
	let session = analytics_ext::session_from_cookies(request.headers());
	let user_agent = request
		.headers()
		.get::<header::UserAgent>()
		.and_then(|res| res.ok());

	let response = next.call(request).await?;
	let status = response.status().as_u16();

	// a geoip country from the address, always derived; the raw ip is stored only
	// when explicitly opted in, keeping the default posture free of personal data.
	let country = ip
		.zip(
			world
				.with(|world: &mut World| world.get_resource::<GeoIp>().cloned())
				.await,
		)
		.and_then(|(ip, geoip)| geoip.country(ip));

	let mut event = AnalyticsEvent::new(path, AnalyticsEventData::Request {
		status,
		method: method.into(),
		user_agent: user_agent.as_deref().map(SmolStr::from),
	})
	.with_client_kind(router_analytics_ext::request_client_kind(
		user_agent.is_some(),
	))
	.with_session(session);
	event.country = country;
	if config.store_ip {
		if let Some(ip) = ip {
			event.ip = Some(ip.to_string().into());
		}
	}

	// fire-and-forget: the analytics observer persists it off the request path.
	world.with(move |world: &mut World| world.trigger(event)).await;

	Ok(response)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// Captures every triggered [`AnalyticsEvent`] for assertions.
	#[derive(Resource, Default, Deref)]
	struct Captured(std::sync::Arc<std::sync::Mutex<Vec<AnalyticsEvent>>>);

	/// A router carrying the config + middleware, a capturing observer, and one
	/// route. Returns the root entity.
	fn analytics_router(world: &mut World) -> Entity {
		world.init_resource::<Captured>();
		let captured = world.resource::<Captured>().0.clone();
		world.add_observer(move |ev: On<AnalyticsEvent>| {
			captured.lock().unwrap().push(ev.event().clone());
		});
		world
			.spawn((
				Router,
				AnalyticsConfig::default(),
				AnalyticsMiddleware::default(),
				children![render_action::fixed_func_route("about", || rsx! {
					<p>"About"</p>
				})],
			))
			.flush()
	}

	#[beet_core::test]
	async fn records_request_event() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = analytics_router(&mut world);
		world
			.entity_mut(root)
			.exchange(
				Request::get("about").with_user_agent("Mozilla/5.0 Test"),
			)
			.await
			.status()
			.xpect_eq(StatusCode::OK);
		// the trigger runs synchronously inside the awaited middleware, so the
		// event is captured by the time exchange resolves.
		let events = world.resource::<Captured>().lock().unwrap().clone();
		events.len().xpect_eq(1);
		let event = &events[0];
		event.event_kind.xpect_eq(AnalyticsEventKind::Request);
		event.client_kind.xpect_eq(ClientKind::Web);
		event.path.as_str().xpect_eq("/about");
		match &event.data {
			AnalyticsEventData::Request {
				status,
				user_agent,
				..
			} => {
				(*status).xpect_eq(200);
				user_agent.as_deref().xpect_eq(Some("Mozilla/5.0 Test"));
			}
			_ => panic!("expected a request event"),
		}
	}

	#[beet_core::test]
	async fn skips_the_analytics_beacon_path() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = analytics_router(&mut world);
		world
			.entity_mut(root)
			.exchange(Request::get("analytics"))
			.await;
		world
			.resource::<Captured>()
			.lock()
			.unwrap()
			.is_empty()
			.xpect_true();
	}
}
