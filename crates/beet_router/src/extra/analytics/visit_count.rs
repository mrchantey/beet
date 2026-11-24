//! The visit count a page shows, read live from the analytics store.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The site's visit count, ie `1234 visits`, read from the store the serving
/// router records analytics to.
///
/// Counts visits (distinct sessions) and says so, never visitors: a session
/// is never linked across visits ([`AnalyticsSummary::visits`]). The store is
/// the one the nearest [`AnalyticsConfig`] above the page's route records to,
/// and the count is `beet analytics`' own, read from the site-wide aggregate
/// of each day and the raw events after them ([`AnalyticsStore::visits`]), a read per day kept, so
/// place it in a [`ServerIsland`] behind a short edge cache, never in a page
/// cached whole.
///
/// ```html
/// <ServerIsland name="visits" {CacheHeaders{edge_max_age: "60s"}}>
/// 	<p><VisitCount/> and counting</p>
/// </ServerIsland>
/// ```
///
/// The read is async content: it parks a [`TemplatePending`] dependency on
/// its build root, which the island step awaits before the island renders. A
/// failed read rides [`TemplateError`] like any build failure, so the island
/// route answers an error and the placeholder keeps its fallback.
#[template(system)]
pub fn VisitCount(
	stack: Res<RequestContextStack>,
	configs: AncestorQuery<(), With<AnalyticsConfig>>,
) -> Result<impl Bundle> {
	let route = stack.current().route();
	let Ok(config) = configs.get_entity(route) else {
		bevybail!(
			"`<VisitCount/>` reads the analytics store its page's router \
			 records to, but no `AnalyticsConfig` sits above route {route}: \
			 declare one on the router, ie \
			 `<Router {{(AnalyticsConfig, StoreRef($analytics))}}>`"
		);
	};
	OnSpawn::new(move |entity| {
		TemplatePending::register_fetch(
			entity,
			PendingKind::Structural,
			"<VisitCount/>",
			move |entity, guard| VisitCount::read(entity, config, guard),
		)
	})
	.xok()
}

impl VisitCount {
	/// Read the summary through `config`'s writer, then spawn the count under
	/// `entity` and resolve the dependency.
	async fn read(entity: AsyncEntity, config: Entity, guard: PendingGuard) {
		let world = entity.world();
		let visits = async {
			StoreRef::resolve::<AnalyticsStore>(&world, config)
				.await?
				.visits()
				.await
		}
		.await;
		let parent = entity.id();
		world
			.with(move |world: &mut World| {
				// a failure rides `TemplateError` like any build failure
				match (visits, world.get_entity_mut(parent)) {
					(Ok(visits), Ok(mut entity)) => {
						let text = Self::text(visits);
						entity.with_child(rsx! { <span>{text}</span> });
					}
					(Err(err), Ok(mut entity)) => {
						entity.insert(TemplateError::new(err));
					}
					// the page was released before the read landed
					(_, Err(_)) => {}
				}
				guard.resolve(world);
			})
			.await;
	}

	/// `visits` worded as visits, singular for one.
	fn text(visits: usize) -> String {
		match visits {
			1 => "1 visit".into(),
			visits => format!("{visits} visits"),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// A page event in session `session`, `days_ago` days back.
	fn view(session: Uuid, days_ago: u64) -> AnalyticsEvent {
		AnalyticsEvent::new("/", AnalyticsEventData::PageView {
			duration_ms: 1_000,
			referrer: None,
			title: None,
			client: default(),
		})
		.with_session(Some(session))
		.with_timestamp(
			Timestamp::now() - Duration::from_secs(days_ago * 86_400),
		)
	}

	/// A router recording analytics to `store`, its home page holding the
	/// count in an island, and the island route.
	fn site(store: InMemoryStore) -> (World, Entity) {
		let mut world = App::new()
			.with_plugins((AsyncPlugin, RouterPlugin))
			.world_mut()
			.xmap(core::mem::take);
		let store = world.spawn(store).id();
		let router = world
			.spawn((
				Router,
				AnalyticsConfig::default(),
				StoreRef(store),
				children![
					render_action::fixed_func_route("", || rsx! {
						<ServerIsland
							name="visits"
							content={|| rsx! { <p><VisitCount/>" and counting"</p> }}
						/>
					}),
					rsx! { <ServerIslands/> },
				],
			))
			.flush();
		(world, router)
	}

	/// The count composes the aggregates of compacted days with the raw tail
	/// of the rest, and is worded as visits.
	#[beet_core::test]
	async fn counts_rollups_and_raw_events() {
		let store = InMemoryStore::new();
		let analytics =
			AnalyticsStore::new(TableStore::new(BlobStore::new(store.clone())));
		// two visits today, one of them reading two pages
		let (left, right) = (uuid_ext::now_v7(), uuid_ext::now_v7());
		for event in [view(left, 0), view(left, 0), view(right, 0)] {
			analytics.record(event).await.unwrap();
		}
		// three visits on a rolled up day, its raw events gone
		let old = (0..3)
			.map(|_| view(uuid_ext::now_v7(), 2))
			.collect::<Vec<_>>();
		for row in AnalyticsRollup::from_events(&old) {
			analytics.rollups().put(row).await.unwrap();
		}

		let (mut world, router) = site(store);
		world
			.entity_mut(router)
			.exchange(
				Request::get("/_server-islands/visits")
					.with_accept(MediaType::Html),
			)
			.await
			.unwrap_str()
			.await
			.xpect_contains("<p><span>5 visits</span> and counting</p>");
	}

	/// A page whose router records no analytics fails its island, naming the
	/// declaration it needs.
	#[beet_core::test]
	async fn needs_an_analytics_config() {
		let (mut world, router) = site(InMemoryStore::new());
		world.entity_mut(router).remove::<AnalyticsConfig>();
		world
			.entity_mut(router)
			.exchange(
				Request::get("/_server-islands/visits")
					.with_accept(MediaType::Html),
			)
			.await
			.into_result()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("no `AnalyticsConfig`");
	}

	#[beet_core::test]
	fn words_visits() {
		VisitCount::text(1).xpect_eq("1 visit");
		VisitCount::text(0).xpect_eq("0 visits");
	}
}
