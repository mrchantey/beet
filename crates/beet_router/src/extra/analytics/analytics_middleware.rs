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
//! itself or fills with load-balancer pings, as are unreferred 404s, which are
//! scanner probes rather than traffic.
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
	// the page this request was linked from, which is what tells a broken link
	// (someone followed a link here) from a probe (a scanner arriving cold).
	let referrer = request
		.headers()
		.get::<header::Referer>()
		.and_then(|res| res.ok());

	let response = next.call(request).await?;
	let status = response.status().as_u16();

	// an unreferred 404 is a scanner walking a vulnerability list, never a visit:
	// nothing linked to the path and nothing served it. On a public site these
	// outnumber every real event several times over, so recording them costs a
	// store write each to bury the signal. A *referred* 404 is the opposite, a
	// link that should have worked, so it is kept (see
	// `AnalyticsSummary::broken_links`). Dropping here also skips the geoip and
	// store work below for the bulk of hostile traffic.
	if status == 404 && referrer.is_none() {
		return Ok(response);
	}

	// a geoip country from the address, always derived; the raw ip is stored only
	// when explicitly opted in, keeping the default posture free of personal data.
	// The database rides the `AnalyticsConfig` entity, resolved by the same
	// ancestry walk as the config itself.
	let country = ip
		.zip(
			world
				.with_state::<AncestorQuery<&GeoIp>, Option<GeoIp>>(move |query| {
					query.get(caller_id).ok().cloned()
				})
				.await,
		)
		.and_then(|(ip, geoip)| geoip.country(ip));

	let mut event = AnalyticsEvent::new(path, AnalyticsEventData::Request {
		status,
		method: method.into(),
		user_agent: user_agent.as_deref().map(SmolStr::from),
		referrer: referrer.as_deref().map(SmolStr::from),
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
	world
		.with(move |world: &mut World| world.trigger(event))
		.await;

	Ok(response)
}

/// The isolation invariant: a stalled analytics store never delays a request.
///
/// This pins the shape of a production incident on the beet site, where batches
/// of about nineteen unrelated requests all completed at the same instant two
/// and a half minutes late, alongside AWS SDK connect timeouts in the same
/// window. The suspicion was that the analytics write had crept onto the request
/// path, so a table that stopped answering would hang every request queued
/// behind it. Both hops that could do that are covered: this middleware only
/// `trigger`s the event, and the persistence observer spawns the push detached.
///
/// The invariant is sharper than it looks, because a beet binary built without
/// `bevy_multithreaded` (the deployed default) runs every detached task on the
/// world-owning thread — see [`AsyncSpawner`](beet_core::prelude::AsyncSpawner).
/// One future that blocks that thread inside `poll` freezes the whole server and
/// releases the entire queue at once, which is exactly the incident's signature.
/// Awaiting the store keeps it out of that class; blocking on it does not, and
/// swapping the stalled store's `sleep` for a `std::thread::sleep` fails it.
///
/// Native-only: the run needs a real listener and real client threads.
#[cfg(all(test, feature = "http", not(target_arch = "wasm32")))]
mod stalled_store_test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::exports::bytes::Bytes;
	use beet_net::prelude::*;
	use std::io::Read;
	use std::io::Write;
	use std::sync::Arc;
	use std::sync::atomic::AtomicUsize;
	use std::sync::atomic::Ordering;

	/// How long a write parks: far beyond [`BUDGET`], so a request that waits on
	/// one cannot possibly come in under budget.
	const STALL: Duration = Duration::from_secs(30);
	/// Wall-clock budget for the whole request batch. Generous: these are trivial
	/// renders, so anything near a second already means the store is on the
	/// request path.
	const BUDGET: Duration = Duration::from_secs(3);
	/// Requests fired concurrently behind the stalled writes. Comfortably above
	/// the ~19 the production batch stalled, so the assertion covers a queue
	/// deeper than the incident's.
	const REQUESTS: usize = 32;

	/// A write-only store whose every write parks for [`STALL`] and never lands,
	/// standing in for a DynamoDB table behind a connect timeout. Reads answer
	/// empty rather than delegating: nothing in this test reads the table, and a
	/// write that never returns is the whole point.
	#[derive(Clone, Component)]
	#[component(on_add = BlobStore::on_add::<Self>)]
	struct StalledStore {
		/// Writes started, ie the count of parked pushes. Non-zero is what proves
		/// the request actually reached the store rather than passing it by.
		started: Arc<AtomicUsize>,
	}

	impl BlobStoreProvider for StalledStore {
		fn box_clone(&self) -> Box<dyn BlobStoreProvider> { Box::new(self.clone()) }
		/// Every path stalls, so a subdir view is the same store.
		fn with_subdir(&self, _path: SmolPath) -> Box<dyn BlobStoreProvider> {
			Box::new(self.clone())
		}
		fn id(&self) -> &'static str { "stalled" }
		fn root_key(&self) -> SmolStr { "stalled".into() }
		fn region(&self) -> Option<String> { None }
		fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
			Box::pin(async { true.xok() })
		}
		fn store_create(&self) -> SendBoxedFuture<Result> {
			Box::pin(async { Ok(()) })
		}
		fn store_remove(&self) -> SendBoxedFuture<Result> {
			Box::pin(async { Ok(()) })
		}
		fn insert(&self, _path: &SmolPath, _body: Bytes) -> SendBoxedFuture<Result> {
			self.started.fetch_add(1, Ordering::SeqCst);
			Box::pin(async {
				time_ext::sleep(STALL).await;
				Ok(())
			})
		}
		fn list(&self) -> SendBoxedFuture<Result<Vec<SmolPath>>> {
			Box::pin(async { Vec::new().xok() })
		}
		fn get(&self, path: &SmolPath) -> SendBoxedFuture<Result<Bytes>> {
			let path = path.clone();
			Box::pin(async move { bevybail!("no row at {path}") })
		}
		fn exists(&self, _path: &SmolPath) -> SendBoxedFuture<Result<bool>> {
			Box::pin(async { false.xok() })
		}
		fn remove(&self, _path: &SmolPath) -> SendBoxedFuture<Result> {
			Box::pin(async { Ok(()) })
		}
		fn public_url(
			&self,
			_path: &SmolPath,
		) -> SendBoxedFuture<Result<Option<String>>> {
			Box::pin(async { None.xok() })
		}
	}

	/// One blocking HTTP/1.1 `GET` on a fresh connection, returning the raw
	/// response text. Deliberately raw std sockets: the assertion is about wall
	/// clock, so the client must not share an executor with anything under test.
	fn raw_get(
		port: u16,
		path: &str,
		timeout: Duration,
	) -> std::io::Result<String> {
		let mut stream = std::net::TcpStream::connect(("127.0.0.1", port))?;
		stream.set_read_timeout(Some(timeout))?;
		write!(
			stream,
			"GET /{path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
		)?;
		let mut response = String::new();
		stream.read_to_string(&mut response)?;
		response.xok()
	}

	/// Serve a router whose analytics store never lands a write, then drive real
	/// requests through it: every one must return promptly, and the store must
	/// have been reached (else the test passes vacuously).
	///
	/// The timeout is raised well past [`BUDGET`] so a regression is reported by
	/// the budget assertion, not swallowed by the runner's default 5s cap.
	#[beet_core::test(timeout_ms = 30_000)]
	async fn stalled_store_never_delays_a_request() {
		let started = Arc::new(AtomicUsize::new(0));
		let (server, on_spawn) =
			HttpServer::new_test(HttpServer::start_mini_with_tcp);
		let port = server.port.unwrap();

		let store_started = started.clone();
		std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, ServerPlugin, RouterPlugin));
			let store = app
				.world_mut()
				.spawn(StalledStore {
					started: store_started,
				})
				.id();
			// the server owns the boot, the analytics router is its dispatch child
			app.world_mut().spawn((server, on_spawn, children![(
				Router,
				AnalyticsConfig::default(),
				TableStoreRef(store),
				AnalyticsMiddleware::default(),
				children![render_action::fixed_func_route("about", || rsx! {
					<p>"About"</p>
				})],
			)]));
			app.run();
		});

		// warm up until a request has actually reached the store: the listener
		// binds and the store resolves through async tasks, so an early request can
		// outrun either. Reaching the store is also the vacuous-pass guard — without
		// it a test that never enabled analytics would trivially pass.
		for _ in 0..40 {
			time_ext::sleep_millis(100).await;
			raw_get(port, "about", BUDGET).ok();
			if started.load(Ordering::SeqCst) > 0 {
				break;
			}
		}
		started.load(Ordering::SeqCst).xpect_greater_than(0);

		// every write from here on is parked forever; unrelated requests behind
		// them must still complete inside the budget. The socket timeout is past
		// the budget so a stall shows up as elapsed time, not a read error.
		let start = Instant::now();
		let clients = (0..REQUESTS)
			.map(|_| {
				std::thread::spawn(move || raw_get(port, "about", BUDGET * 3))
			})
			.collect::<Vec<_>>()
			.into_iter()
			.map(|client| client.join().unwrap())
			.collect::<Vec<_>>();
		start.elapsed().xpect_less_than(BUDGET);
		for client in clients {
			client
				.unwrap()
				.xpect_contains("200 OK")
				.xpect_contains("About");
		}
	}
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
		let store = world.spawn(InMemoryStore::new()).flush();
		world
			.spawn((
				Router,
				AnalyticsConfig::default(),
				TableStoreRef(store),
				AnalyticsMiddleware::default(),
				children![render_action::fixed_func_route("about", || rsx! {
					<p>"About"</p>
				})],
			))
			.flush()
	}

	/// Exchange `request` against a fresh analytics router, returning the response
	/// status and every event the middleware recorded.
	///
	/// The trigger runs synchronously inside the awaited middleware, so the
	/// events are captured by the time the exchange resolves.
	async fn exchange(request: Request) -> (StatusCode, Vec<AnalyticsEvent>) {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = analytics_router(&mut world);
		let status = world.entity_mut(root).exchange(request).await.status();
		let events = world.resource::<Captured>().lock().unwrap().clone();
		(status, events)
	}

	#[beet_core::test]
	async fn records_request_event() {
		let (status, events) =
			exchange(Request::get("about").with_user_agent("Mozilla/5.0 Test"))
				.await;
		status.xpect_eq(StatusCode::OK);
		events.len().xpect_eq(1);
		let event = &events[0];
		event.event_kind.xpect_eq(AnalyticsEventKind::Request);
		event.client_kind.xpect_eq(ClientKind::Web);
		event.path.as_str().xpect_eq("/about");
		match &event.data {
			AnalyticsEventData::Request {
				status, user_agent, ..
			} => {
				(*status).xpect_eq(200);
				user_agent.as_deref().xpect_eq(Some("Mozilla/5.0 Test"));
			}
			_ => panic!("expected a request event"),
		}
	}

	/// The referrer rides the request event: it is what tells a broken link from a
	/// probe when the response is a 404 (see `AnalyticsSummary::broken_links`).
	#[beet_core::test]
	async fn records_the_referrer() {
		let (_, events) = exchange(
			Request::get("about")
				.with_header::<header::Referer>("https://beet.org/docs"),
		)
		.await;
		match &events[0].data {
			AnalyticsEventData::Request { referrer, .. } => {
				referrer.as_deref().xpect_eq(Some("https://beet.org/docs"));
			}
			_ => panic!("expected a request event"),
		}
	}

	/// An unreferred 404 is a scanner probe, not traffic, and on a public site
	/// they were 82% of every recorded event, ie 82% of the store writes, for a
	/// stream nothing reads. A *referred* 404 is a broken link and must survive.
	#[beet_core::test]
	async fn skips_unreferred_404s() {
		// a cold 404: dropped.
		let (status, events) = exchange(Request::get("wp-login.php")).await;
		status.xpect_eq(StatusCode::NOT_FOUND);
		events.is_empty().xpect_true();

		// a linked 404: kept, it is a broken link on the site.
		let (status, events) = exchange(
			Request::get("docs/typo")
				.with_header::<header::Referer>("https://beet.org/docs"),
		)
		.await;
		status.xpect_eq(StatusCode::NOT_FOUND);
		events.len().xpect_eq(1);
		match &events[0].data {
			AnalyticsEventData::Request {
				status, referrer, ..
			} => {
				(*status).xpect_eq(404);
				referrer.as_deref().xpect_eq(Some("https://beet.org/docs"));
			}
			_ => panic!("expected a request event"),
		}

		// a served page is kept whether or not anything linked to it.
		for request in [
			Request::get("about"),
			Request::get("about")
				.with_header::<header::Referer>("https://beet.org/docs"),
		] {
			let (status, events) = exchange(request).await;
			status.xpect_eq(StatusCode::OK);
			events.len().xpect_eq(1);
		}
	}

	#[beet_core::test]
	async fn skips_the_analytics_beacon_path() {
		let (_, events) = exchange(Request::get("analytics")).await;
		events.is_empty().xpect_true();
	}
}
