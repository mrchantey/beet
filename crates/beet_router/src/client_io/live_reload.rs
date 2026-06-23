//! Dev-mode live reload: subscribe to the site store, refresh the world, tell
//! clients.
//!
//! The trigger is a [`BlobEvent`], not a filesystem watcher: the site store's own
//! watcher (fs notify, in-memory broadcast, localStorage) emits one, drained into
//! the global `On<BlobEvent>` by [`StorePlugin`]. So an in-memory or remote store
//! drives reloads the same way a local dir does, and nothing here touches the
//! filesystem directly.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_net::sockets::Message;
#[cfg(test)]
use beet_net::sockets::MessageSend;

/// The [`ClientIoBroadcast`] payload instructing clients to reload the page.
pub const RELOAD_MESSAGE: &str = "reload";

/// Dev-mode live reload for a no-code site, placed on the site root (which carries
/// the site [`BlobStore`]). Any change to that store surfaces as a [`BlobEvent`]
/// (emitted by the store's watcher), which re-registers the site's `templates/`
/// through the store, respawns every [`RoutesDir`]'s routes, and broadcasts
/// [`RELOAD_MESSAGE`] over the world's [`ClientIo`] channel (spawned as a child if
/// none exists). The [`LiveReloadScript`](super::LiveReloadScript) widget turns the
/// broadcast into a browser reload.
///
/// Store-agnostic: the reload reads the site content through the resolved store, so
/// a filesystem, in-memory, or remote backing all reload identically.
#[derive(Debug, Clone, Component)]
pub struct LiveReload {
	/// Store paths matching an exclude never trigger a reload; defaults to the
	/// `.git`/`dist`/`target` churn a served site never edits.
	pub filter: GlobFilter,
}

impl Default for LiveReload {
	fn default() -> Self {
		Self {
			filter: GlobFilter::default()
				.with_exclude("*.git*")
				.with_exclude("*dist*")
				.with_exclude("*target*"),
		}
	}
}

impl LiveReload {
	/// A live reload with the default exclude filter.
	pub fn new() -> Self { Self::default() }
}

/// Marks a [`LiveReload`] site as having a pending change, coalescing a burst of
/// [`BlobEvent`]s into a single reload per tick (insert is idempotent).
#[derive(Component)]
pub(crate) struct NeedsReload;

/// Marks a [`LiveReload`] site whose async reload is in flight, so further events
/// queue (via [`NeedsReload`]) rather than racing a second overlapping reload.
#[derive(Component)]
pub(crate) struct Reloading;

/// Observer: a spawned [`LiveReload`] gets a child [`ClientIo`] channel if the
/// world has none (its store's watcher already emits the change events, so no
/// per-site filesystem watcher is wired here).
pub(crate) fn start_live_reload(
	ev: On<Insert, LiveReload>,
	channels: Query<&ClientIo>,
	mut commands: Commands,
) {
	if channels.is_empty() {
		commands.spawn((ChildOf(ev.entity), ClientIo));
	}
}

/// Observer: a [`BlobEvent`] landed; mark every [`LiveReload`] site whose store
/// owns the changed object (minus excluded churn) as needing a reload.
pub(crate) fn reload_site_on_change(
	ev: On<BlobEvent>,
	sites: Query<(Entity, &LiveReload, &BlobStore)>,
	mut commands: Commands,
) {
	sites
		.iter()
		.filter(|(_, site, store)| {
			store.did_change(ev.event()) && site.filter.passes(ev.path.as_str())
		})
		.for_each(|(entity, _, _)| {
			debug!("site store changed, reloading: {}", ev.path);
			commands.entity(entity).insert(NeedsReload);
		});
}

/// Drive the pending reloads once per tick: each [`NeedsReload`] site not already
/// [`Reloading`] is refreshed, the markers sequencing coalesce-and-no-overlap.
pub(crate) fn process_live_reloads(world: &mut World) {
	let roots = world.with_state::<Query<
		Entity,
		(With<LiveReload>, With<NeedsReload>, Without<Reloading>),
	>, _>(|query| query.iter().collect::<Vec<_>>());
	for root in roots {
		world
			.entity_mut(root)
			.remove::<NeedsReload>()
			.insert(Reloading);
		reload_site(world, root);
	}
}

/// Refresh the world from the site's [`BlobStore`]: re-register its `templates/`,
/// respawn every [`RoutesDir`]'s route children (rebuilding the route trees), then
/// broadcast [`RELOAD_MESSAGE`] to connected clients. `root` is the [`LiveReload`]
/// entity carrying the store; releases its [`Reloading`] guard when done.
pub fn reload_site(world: &mut World, root: Entity) {
	let Some(store) = world.entity(root).get::<BlobStore>().cloned() else {
		warn!("live reload root {root} has no BlobStore");
		world.entity_mut(root).remove::<Reloading>();
		return;
	};
	let formats = world.get_resource_or_init::<TemplateFormats>().clone();
	// the in-world TUI navigators (no `ClientIo` client) to repaint directly.
	let navigators = in_world_navigators(world);
	world.run_async(move |world| async move {
		// re-register `templates/` through the store so the registry serves the
		// edited sources (store-agnostic: fs, in-memory, remote all work).
		let sources = read_site_templates(
			&store,
			&formats,
			&SmolPath::from(DEFAULT_TEMPLATES_DIR),
		)
		.await?;
		world
			.with(move |world: &mut World| -> Result {
				register_site_templates(world, &formats, sources)?;
				respawn_routes_dirs(world);
				broadcast_reload(world);
				Ok(())
			})
			.await?;

		// the dev loop: settle the async rescan (rendering a half-scanned tree would
		// paint stale content), surface render diagnostics (an unknown tag, dead link
		// or unknown class an edit introduced logs loudly), then repaint each in-world
		// navigator *after* the diagnostics so its freshly-built page is the last
		// render of each shared node (else the diagnostics' ephemeral cleanup races
		// the repaint and blanks the live TUI). The web client has no in-world
		// navigator and reloads via the broadcast above.
		settle_routes_dirs(&world).await?;
		log_all_render_diagnostics(&world).await;
		for navigator in navigators {
			if let Err(err) = Navigator::reload(world.entity(navigator)).await {
				error!("live reload repaint failed: {err}");
			}
		}
		// release the guard; a change that landed mid-reload left `NeedsReload`, so
		// the next tick reloads again.
		world
			.with(move |world: &mut World| {
				world.entity_mut(root).remove::<Reloading>();
			})
			.await;
		Ok(())
	});
}

/// Respawn every [`RoutesDir`]'s route children: re-inserting the `RoutesDir`
/// re-fires the async discovery observer, which respawns the routes and rebuilds the
/// tree. The scoped [`BlobStore`] is dropped too, so the rescan's completion (it
/// re-composes the store) is observable via [`settle_routes_dirs`].
fn respawn_routes_dirs(world: &mut World) {
	let dirs = world.with_state::<Query<(Entity, &RoutesDir)>, _>(|query| {
		query
			.iter()
			.map(|(entity, dir)| (entity, dir.clone()))
			.collect::<Vec<_>>()
	});
	for (entity, dir) in dirs {
		world
			.entity_mut(entity)
			.despawn_related::<Children>()
			.remove::<BlobStore>()
			.insert(dir);
	}
	world.flush();
}

/// Broadcast [`RELOAD_MESSAGE`] to every connected [`ClientIo`] client.
fn broadcast_reload(world: &mut World) {
	let channels =
		world.with_state::<Query<Entity, With<ClientIo>>, _>(|query| {
			query.iter().collect::<Vec<_>>()
		});
	for channel in channels {
		world
			.entity_mut(channel)
			.trigger_target(ClientIoBroadcast(Message::text(RELOAD_MESSAGE)));
	}
}

/// The in-world [`Navigator`] entities (the live-TUI navigators that browse the
/// app's own routes with no socket client), to repaint on reload as the TUI
/// counterpart of the [`ClientIo`] reload broadcast.
///
/// Empty for an HTTP-only app, so the reload repaint is inert outside the live TUI.
fn in_world_navigators(world: &mut World) -> Vec<Entity> {
	world.with_state::<Query<(Entity, &Navigator)>, _>(|query| {
		query
			.iter()
			.filter(|(_, nav)| {
				matches!(nav.transport(), NavigatorTransport::InWorld { .. })
			})
			.map(|(entity, _)| entity)
			.collect()
	})
}

#[cfg(test)]
mod test {
	use super::*;
	use beet_ui::prelude::*;

	/// A router app with the live-reload observers + reload system, plus the
	/// `PackageConfig` the reload's render diagnostics read (see `site_layout`).
	fn reload_app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin, RouterPlugin))
			.init_resource::<PackageConfig>();
		app
	}

	/// Write a site fixture (`templates/` + `routes/`) under `target/tests` and
	/// return its root dir.
	fn site_fixture(name: &str) -> AbsPathBuf {
		let root = AbsPathBuf::new(
			fs_ext::workspace_root()
				.join("target/tests/live_reload")
				.join(name),
		)
		.unwrap();
		// clean slate so files from previous runs do not leak in
		fs_ext::remove(&root).ok();
		fs_ext::write(
			root.join("templates/Card.bsx"),
			"<section>first card</section>",
		)
		.unwrap();
		fs_ext::write(root.join("routes/index.md"), "# Home\n\nwelcome")
			.unwrap();
		root
	}

	/// Spawn a watched site: a router serving `routes/` from `store` (an `FsStore`,
	/// `InMemoryStore`, ...), marked [`LiveReload`] so `start_live_reload` gives it a
	/// [`ClientIo`] child. Returns the root entity, which carries the store.
	fn spawn_site(world: &mut World, store: impl Bundle) -> Entity {
		world
			.spawn((store, default_router(), LiveReload::new(), children![
				RoutesDir::new("routes")
			]))
			.flush()
	}

	/// Editing templates and adding a route, then reloading, re-registers the site's
	/// `templates/` through the store and respawns the routes. Reads the edits back
	/// through the `FsStore`, so nothing here touches the filesystem after the writes.
	#[beet_core::test]
	async fn reload_reregisters_templates_and_respawns_routes() {
		let mut app = reload_app();
		let site_dir = site_fixture("respawns");
		let root = spawn_site(app.world_mut(), FsStore::new(site_dir.clone()));
		// the RoutesDir scan is async, so settle it before reading the tree
		AsyncRunner::settle_async_tasks(app.world_mut()).await;
		let routes_dir = app
			.world_mut()
			.with_state::<Query<Entity, With<RoutesDir>>, _>(|query| {
				query.single().unwrap()
			});
		app.world()
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["docs"])
			.xpect_none();

		// mutate the site: a new route, an edited template, a new template
		fs_ext::write(site_dir.join("routes/docs/intro.md"), "# Intro").unwrap();
		fs_ext::write(
			site_dir.join("templates/Card.bsx"),
			"<section>second card</section>",
		)
		.unwrap();
		fs_ext::write(site_dir.join("templates/Hero.bsx"), "<h1>hero</h1>")
			.unwrap();
		// reload the site (the store read picks up the edits); the async reload then
		// re-registers templates and respawns the routes, so settle it.
		reload_site(app.world_mut(), root);
		AsyncRunner::settle_async_tasks(app.world_mut()).await;

		// the new route landed in the rebuilt tree
		app.world()
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["docs", "intro"])
			.xpect_some();
		// the old routes respawned exactly once
		app.world()
			.entity(routes_dir)
			.get::<Children>()
			.unwrap()
			.len()
			.xpect_eq(2);
		// the registry serves the edited and the new template sources
		let registry = app.world().resource::<BsxTemplateRegistry>();
		registry.contains("Hero").xpect_true();
		registry
			.get("Card")
			.unwrap()
			.nodes
			.xref()
			.xmap(|nodes| format!("{nodes:?}"))
			.xpect_contains("second card");
	}

	/// The store drives reloads, not the filesystem: an [`InMemoryStore`]'s own
	/// watcher emits a [`BlobEvent`] on a write, which flows through the global event
	/// pipeline ([`StorePlugin`]) into a reload that rescans the store. Proves the
	/// in-memory watcher path and the store-agnostic live-reload integration.
	#[beet_core::test]
	async fn blob_event_drives_in_memory_reload() {
		let mut app = reload_app();
		let store = InMemoryStore::new();
		// seed an initial route, then spawn the site over the same backing
		let handle = BlobStore::new(store.clone());
		handle
			.insert(&SmolPath::from("routes/index.md"), "# Home")
			.await
			.unwrap();
		let root = spawn_site(app.world_mut(), store);
		// subscribe the in-memory watcher + settle the initial RoutesDir scan
		app.update();
		AsyncRunner::settle_async_tasks(app.world_mut()).await;
		app.world()
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["about"])
			.xpect_none();

		// add a route through the store: the in-memory watcher emits a `BlobEvent`,
		// drained next update into the reload-on-change observer.
		handle
			.insert(&SmolPath::from("routes/about.md"), "# About")
			.await
			.unwrap();
		// drain the event (PreUpdate) -> mark NeedsReload -> reload (Update), then
		// settle the async rescan.
		app.update();
		AsyncRunner::settle_async_tasks(app.world_mut()).await;
		// the reload rescanned the store, so the new route is in the rebuilt tree
		app.world()
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["about"])
			.xpect_some();
	}

	#[beet_core::test]
	async fn broadcasts_reload_to_clients() {
		let mut app = reload_app();
		let site_dir = site_fixture("broadcasts");
		let root = spawn_site(app.world_mut(), FsStore::new(site_dir.clone()));
		AsyncRunner::settle_async_tasks(app.world_mut()).await;
		// the `ClientIo` channel `start_live_reload` spawned as the root's child
		let channel = app
			.world_mut()
			.with_state::<Query<Entity, With<ClientIo>>, _>(|query| {
				query.single().unwrap()
			});
		let received = Store::<Vec<Message>>::default();
		let captor = received.clone();
		app.world_mut().spawn(ChildOf(channel)).observe_any(
			move |ev: On<MessageSend>| {
				captor.push(ev.event().inner().clone());
			},
		);

		reload_site(app.world_mut(), root);
		AsyncRunner::settle_async_tasks(app.world_mut()).await;

		received.get().xpect_eq(vec![Message::text(RELOAD_MESSAGE)]);
	}

	#[beet_core::test]
	fn spawns_a_channel_when_none_exists() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		// no pre-set ClientIo: `start_live_reload` spawns one as the root's child
		let root = world
			.spawn((InMemoryStore::new(), default_router(), LiveReload::new()))
			.flush();
		let channel = world
			.with_state::<Query<Entity, With<ClientIo>>, _>(|query| {
				query.single()
			})
			.unwrap();
		world
			.entity(channel)
			.get::<ChildOf>()
			.unwrap()
			.parent()
			.xpect_eq(root);
	}

	/// A deck fixture: zero-padded card files under `slides/` (deliberately
	/// out-of-order on disk) backing a [`CardDeck`] router. Returns the site dir.
	fn deck_fixture(name: &str) -> AbsPathBuf {
		let root = AbsPathBuf::new(
			fs_ext::workspace_root()
				.join("target/tests/live_reload")
				.join(name),
		)
		.unwrap();
		fs_ext::remove(&root).ok();
		fs_ext::write(root.join("slides/02-beta.md"), "# Beta").unwrap();
		fs_ext::write(root.join("slides/01-alpha.md"), "# Alpha first")
			.unwrap();
		root
	}

	/// The card path segments of a router's [`RouteTree`], in child order.
	fn card_order(world: &mut World, router: Entity) -> Vec<String> {
		world
			.entity(router)
			.get::<RouteTree>()
			.unwrap()
			.children
			.iter()
			.filter(|child| child.node().is_some())
			.filter_map(|child| child.path.iter().last())
			.map(|seg| seg.name().to_string())
			.collect()
	}

	/// A live reload of a deck router preserves its [`CardDeck`] marker and keeps
	/// the cards in sorted order: the respawn replaces the route children, not the
	/// router, so the marker survives and the rebuilt tree is still ordered.
	#[beet_core::test]
	async fn reload_preserves_card_deck_marker_and_order() {
		let mut app = reload_app();
		let site_dir = deck_fixture("deck_marker");
		// a deck router marked for live reload: the CardDeck marker (declared in the
		// deck's markup spread); the site store on the root backs the `RoutesDir` scan
		// by ancestry.
		let router = app
			.world_mut()
			.spawn((
				FsStore::new(site_dir.clone()),
				Router,
				CardDeck,
				LiveReload::new(),
				children![RoutesDir::new("slides")],
			))
			.flush();
		// the RoutesDir scan is async, so settle it before reading the tree
		AsyncRunner::settle_async_tasks(app.world_mut()).await;
		card_order(app.world_mut(), router)
			.xpect_eq(vec!["01-alpha".to_string(), "02-beta".to_string()]);

		// a new card, then a live reload (the store-change path).
		fs_ext::write(site_dir.join("slides/03-gamma.md"), "# Gamma").unwrap();
		reload_site(app.world_mut(), router);
		// the respawn re-scans each RoutesDir asynchronously, so settle again
		AsyncRunner::settle_async_tasks(app.world_mut()).await;

		// the marker survived the route respawn ...
		app.world().entity(router).contains::<CardDeck>().xpect_true();
		// ... and the rebuilt tree still lists the cards in sorted order.
		card_order(app.world_mut(), router).xpect_eq(vec![
			"01-alpha".to_string(),
			"02-beta".to_string(),
			"03-gamma".to_string(),
		]);
	}

	/// The live TUI stack: charcell pipeline + per-frame repaint + in-world nav.
	/// `RouterPlugin` brings the charcell/template/async plugins and the live
	/// reload observers; the page host + navigator need the realtime/nav plugins.
	fn tui_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			RouterPlugin,
			RealtimeParsePlugin,
			LivePagePlugin,
			NavigatorPlugin,
		));
		// the reload's render diagnostics paint the layout chrome (header/sidebar)
		// outside the request middleware that normally seeds it, so a bare live
		// render world must seed `PackageConfig` itself (see `site_layout`).
		app.init_resource::<PackageConfig>();
		app
	}

	/// Drive the app until the host frame contains `needle`, returning the frame.
	///
	/// Each frame updates the app, ticks the async runtime, then yields a slice of
	/// real time so the route scan + repaint's blocking store I/O can land. The
	/// painted buffer is snapshotted between frames so the first match is caught
	/// before any later repaint can blank it. Rather than a fixed time budget (a
	/// busy shared task pool can stretch the async work arbitrarily), failure is
	/// concluded only once the runtime has gone idle for several frames with the
	/// needle still absent, ie the repaint has landed and will never show it.
	async fn drive_until(app: &mut App, host: Entity, needle: &str) -> String {
		let mut idle_without_match = 0;
		for _ in 0..10_000 {
			app.update();
			AsyncRunner::tick().await;
			time_ext::sleep_millis(1).await;
			let frame = app
				.world()
				.get::<DoubleBuffer>(host)
				.unwrap()
				.current_buffer()
				.render_plain();
			if frame.contains(needle) {
				return frame;
			}
			idle_without_match =
				if app.world().resource::<AsyncSpawner>().in_flight() == 0 {
					idle_without_match + 1
				} else {
					0
				};
			if idle_without_match >= 16 {
				break;
			}
		}
		panic!("host frame never contained '{needle}'");
	}

	/// Editing a card repaints the live terminal: after the watched change
	/// respawns the routes, the in-world navigator re-fetches its current card and
	/// the page host paints the edited content, with the `CardDeck` marker intact.
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	async fn tui_reload_repaints_current_card() {
		use bevy::math::UVec2;

		let mut app = tui_app();
		let site_dir = deck_fixture("tui_repaint");
		// the site store on the router root backs the `RoutesDir` scan by ancestry.
		let router = app
			.world_mut()
			.spawn((
				FsStore::new(site_dir.clone()),
				Router,
				CardDeck,
				LiveReload::new(),
				children![RoutesDir::new("slides")],
			))
			.flush();
		// settle the async RoutesDir scan so the route tree exists before the
		// navigator resolves its initial page; otherwise the navigator's one-shot
		// initial load hits the not-yet-built tree, paints the error page, and never
		// retries (the TUI boot settles the same way before composing the host).
		AsyncRunner::settle_async_tasks(app.world_mut()).await;
		// the host with its in-world navigator co-located, opened on the first card,
		// as the TUI boot composes them.
		let host = app
			.world_mut()
			.spawn((
				page_host(UVec2::new(40, 8)),
				Navigator::in_world(router, "/01-alpha"),
			))
			.id();
		drive_until(&mut app, host, "Alpha first").await;

		// edit the current card on disk, then drive the store-change reload.
		fs_ext::write(site_dir.join("slides/01-alpha.md"), "# Alpha edited")
			.unwrap();
		app.world_mut()
			.commands()
			.queue(move |world: &mut World| reload_site(world, router));
		// the navigator re-fetches the current card and the host repaints it.
		drive_until(&mut app, host, "Alpha edited")
			.await
			.xnot()
			.xpect_contains("Alpha first");
		// the marker survived the respawn, so card nav still works.
		app.world()
			.entity(router)
			.contains::<CardDeck>()
			.xpect_true();
	}

	/// Card nav clamps at the first card and steps without skipping: prev on the
	/// opening card stays put (a deck does not wrap), next then advances to the
	/// second card, and prev returns to the first. Regression for prev stepping
	/// "past" the first card (onto the home or an error page) and a following next
	/// then skipping it.
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	async fn card_nav_clamps_at_first_and_steps_without_skip() {
		use bevy::input::ButtonState;
		use bevy::input::keyboard::Key;
		use bevy::input::keyboard::KeyCode;
		use bevy::input::keyboard::KeyboardInput;
		use bevy::math::UVec2;

		let mut app = tui_app();
		app.add_plugins(CardStackPlugin);
		let site_dir = deck_fixture("deck_nav");
		// the site store on the root backs the `RoutesDir` scan by ancestry.
		let router = app
			.world_mut()
			.spawn((
				FsStore::new(site_dir.clone()),
				Router,
				CardDeck,
				children![RoutesDir::new("slides")],
			))
			.flush();
		AsyncRunner::settle_async_tasks(app.world_mut()).await;
		let host = app
			.world_mut()
			.spawn((
				page_host(UVec2::new(40, 8)),
				Navigator::in_world(router, "/01-alpha"),
			))
			.id();
		drive_until(&mut app, host, "Alpha first").await;

		// the card the navigator currently shows (its path segments joined).
		fn current_card(app: &App, host: Entity) -> String {
			app.world()
				.entity(host)
				.get::<Navigator>()
				.unwrap()
				.current_url()
				.path()
				.join("/")
		}
		// press an arrow (card_nav reads `key_code`), then settle card_nav's queued
		// async navigation over a few frames.
		async fn press(app: &mut App, host: Entity, code: KeyCode) {
			app.world_mut().write_message(KeyboardInput {
				key_code: code,
				logical_key: Key::ArrowLeft,
				state: ButtonState::Pressed,
				text: None,
				repeat: false,
				window: host,
			});
			for _ in 0..40 {
				app.update();
				AsyncRunner::tick().await;
				time_ext::sleep_millis(1).await;
			}
		}

		// prev on the first card clamps: still Alpha, never the home or beta.
		press(&mut app, host, KeyCode::ArrowLeft).await;
		current_card(&app, host).xpect_eq("01-alpha");
		// next advances to the second card (not skipped).
		press(&mut app, host, KeyCode::ArrowRight).await;
		current_card(&app, host).xpect_eq("02-beta");
		// prev returns to the first card.
		press(&mut app, host, KeyCode::ArrowLeft).await;
		current_card(&app, host).xpect_eq("01-alpha");
	}
}
