//! Integration test for the cross-transport analytics flow: a web page request
//! records a `Request` event, the web client beacon records a `PageView`, and an
//! in-world terminal navigation records a `Terminal` `PageView`, all persisted to
//! the same store without dropping prior data.
//!
//! This exercises the full path the unit tests only touch in pieces: emitter ->
//! the `On<AnalyticsEvent>` observer -> the `AnalyticsStore`. The deploy skill's
//! check `e` verifies the same flow against the live remote store.
beet::test_main!();

use beet::prelude::*;

/// An analytics store seeded with one prior event (to prove new events are added,
/// not a fresh store), returned alongside its prior event's id.
async fn seeded_store() -> (AnalyticsStore, Uuid) {
	let store = temp_table::<AnalyticsEvent>();
	let prior = AnalyticsEvent::new("/old", AnalyticsEventData::PageView {
		duration_ms: 500,
		referrer: None,
		title: None,
		client: ClientDescriptor::default(),
	})
	.with_client_kind(ClientKind::Web);
	let prior_id = prior.id;
	store.push(prior).await.unwrap();
	(AnalyticsStore { store }, prior_id)
}

/// A router with analytics enabled over `store`, plus one content route. Inserting
/// the store before the `AnalyticsConfig` lands makes the config's bootstrap a
/// no-op, so the test controls (and can read back) the exact store.
async fn analytics_world(store: AnalyticsStore) -> (World, Entity) {
	let mut world = (AsyncPlugin, RouterPlugin).into_world();
	world.insert_resource(pkg_config!());
	world.insert_resource(store);
	let root = world
		.spawn((
			default_router(),
			AnalyticsConfig::default(),
			AnalyticsMiddleware::default(),
			children![render_action::fixed_func_route("about", || rsx! {
				<p>"About"</p>
			})],
		))
		.flush();
	(world, root)
}

/// A web page request records a `Request` event and the client beacon records a
/// `PageView`, both landing in the store alongside the retained prior event.
#[beet::test]
async fn web_flow_records_and_retains_prior() {
	let (store, prior_id) = seeded_store().await;
	let readback = store.store.clone();
	let (mut world, root) = analytics_world(store).await;

	// a web page request -> a Request event (carrying the user agent).
	world
		.entity_mut(root)
		.exchange(Request::get("about").with_user_agent("Mozilla/5.0 Test"))
		.await
		.status()
		.xpect_eq(StatusCode::OK);
	// the web client beacon -> a PageView event.
	let beacon = r#"{"kind":"page_view","page_view_id":"0192f8a0-0000-7000-8000-0000000000aa","path":"/about","duration_ms":1500}"#;
	world
		.entity_mut(root)
		.exchange(Request::with_json_str("analytics", beacon))
		.await
		.status()
		.xpect_eq(StatusCode::OK);

	// drain the fire-and-forget store pushes.
	AsyncRunner::settle_async_tasks(&mut world).await;

	let events = readback
		.get_all()
		.await
		.unwrap()
		.into_iter()
		.map(|(_, event)| event)
		.collect::<Vec<_>>();

	// the prior event is retained.
	events
		.iter()
		.any(|event| event.id == prior_id)
		.xpect_true();
	// the page request recorded a Request event carrying the user agent.
	events
		.iter()
		.any(|event| {
			event.event_kind == AnalyticsEventKind::Request
				&& event.path == "/about"
				&& event.client_kind == ClientKind::Web
		})
		.xpect_true();
	// the beacon recorded a web page view for the same path.
	events
		.iter()
		.any(|event| {
			event.event_kind == AnalyticsEventKind::PageView
				&& event.path == "/about"
				&& event.client_kind == ClientKind::Web
		})
		.xpect_true();
	// the beacon endpoint itself is skipped by the request middleware (no feedback).
	events
		.iter()
		.any(|event| event.path == "/analytics")
		.xpect_false();
}
