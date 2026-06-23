//! Dev-mode live reload: watch the site dir, refresh the world, tell clients.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_net::sockets::Message;
#[cfg(test)]
use beet_net::sockets::MessageSend;

/// The [`ClientIoBroadcast`] payload instructing clients to reload the page.
pub const RELOAD_MESSAGE: &str = "reload";

/// Dev-mode live reload for a no-code site: watches [`Self::site_dir`] and on
/// any change re-registers the site's `templates/`, respawns every
/// [`RoutesDir`]'s routes, and broadcasts [`RELOAD_MESSAGE`] over the world's
/// [`ClientIo`] channel (spawned as a child if none exists). The
/// [`LiveReloadScript`](super::LiveReloadScript) widget turns the broadcast
/// into a browser reload.
#[derive(Debug, Clone, Component)]
pub struct LiveReload {
	/// The watched site directory, containing the entry, `templates/` and the
	/// content routes.
	pub site_dir: AbsPathBuf,
}

impl LiveReload {
	/// Watch `site_dir` for changes.
	pub fn new(site_dir: AbsPathBuf) -> Self { Self { site_dir } }
}

/// Observer: wire up a spawned [`LiveReload`] with its [`FsWatcher`] and, if
/// the world has none, a child [`ClientIo`] channel.
pub(crate) fn start_live_reload(
	ev: On<Insert, LiveReload>,
	sites: Query<&LiveReload>,
	channels: Query<&ClientIo>,
	mut commands: Commands,
) -> Result {
	let site = sites.get(ev.entity)?;
	commands.entity(ev.entity).insert(
		FsWatcher::new(site.site_dir.clone()).with_filter(
			GlobFilter::default()
				.with_exclude("*.git*")
				.with_exclude("*dist*")
				.with_exclude("*target*"),
		),
	);
	if channels.is_empty() {
		commands.spawn((ChildOf(ev.entity), ClientIo));
	}
	Ok(())
}

/// Observer: a watched site changed, refresh the world and notify clients.
pub(crate) fn reload_site_on_change(
	ev: On<DirEvent>,
	sites: Query<&LiveReload>,
	mut commands: Commands,
) {
	let Ok(site) = sites.get(ev.target()).cloned() else {
		// a DirEvent from some other watcher
		return;
	};
	debug!("site changed, reloading:\n{}", ev.event());
	commands.queue(move |world: &mut World| reload_site(world, &site));
}

/// Refresh the world from disk: re-register the site's `templates/` (if any),
/// despawn and respawn every [`RoutesDir`]'s route children (rebuilding the
/// route trees), then broadcast [`RELOAD_MESSAGE`] to connected clients.
pub fn reload_site(world: &mut World, site: &LiveReload) -> Result {
	// re-register templates so the registry serves the edited sources
	let templates = site.site_dir.join("templates");
	if fs_ext::exists(&templates)? {
		world.register_bsx_templates(&templates)?;
	}
	// respawn each RoutesDir's routes: re-inserting the `RoutesDir` re-fires the
	// async discovery observer, which respawns the route children and rebuilds the
	// tree. The scoped `BlobStore` is also dropped so the rescan's completion (it
	// re-composes the store) is observable via `settle_routes_dirs`.
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

	// the in-world TUI navigator has no `ClientIo` client, so repaint it directly:
	// re-fetch its current page through the rebuilt route tree and the page host
	// repaints. The web client has no in-world navigator and reloads via the
	// broadcast below instead.
	// The discovery rescan is async, so first await it (`settle_routes_dirs`):
	// rendering against a half-scanned tree would paint stale content. Then dev-loop
	// "type-check" then repaint, sequenced in one task: first re-render every route
	// to surface problems (an unknown tag, dead link or unknown class an edit
	// introduced logs loudly), then repaint each in-world navigator. The repaint runs
	// *after* the diagnostics so a navigator's freshly-built page is the last render
	// of each shared route node; otherwise the diagnostics' ephemeral cleanup races
	// the repaint and blanks the live TUI. The web client has no in-world navigator
	// and repaints via the broadcast below. Fire-and-forget (route rendering is
	// async), so the reload never blocks.
	let navigators = in_world_navigators(world);
	world.run_async(move |world| async move {
		if let Err(err) = settle_routes_dirs(&world).await {
			error!("live reload route rescan failed: {err}");
		}
		log_all_render_diagnostics(&world).await;
		for navigator in navigators {
			if let Err(err) = Navigator::reload(world.entity(navigator)).await {
				error!("live reload repaint failed: {err}");
			}
		}
	});

	// tell connected clients to reload
	let channels =
		world.with_state::<Query<Entity, With<ClientIo>>, _>(|query| {
			query.iter().collect::<Vec<_>>()
		});
	for channel in channels {
		world
			.entity_mut(channel)
			.trigger_target(ClientIoBroadcast(Message::text(RELOAD_MESSAGE)));
	}
	Ok(())
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

	/// Write a watched site fixture (`templates/` + `routes/`) under
	/// `target/tests` and return its root.
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

	/// Spawn the watched site: a router serving `routes/`, the registered
	/// templates, and a [`LiveReload`] with its [`ClientIo`] channel (which now
	/// rides the main HTTP port, so no per-channel port to set).
	fn spawn_site(
		world: &mut World,
		site_dir: &AbsPathBuf,
	) -> (Entity, Entity) {
		world
			.register_bsx_templates(site_dir.join("templates"))
			.unwrap();
		// the reload's render diagnostics paint the layout chrome, which reads
		// `PackageConfig`; seed it so the bare render world has it (see `site_layout`).
		world.init_resource::<PackageConfig>();
		// compose the site store on the router root so `RoutesDir` resolves it by ancestry.
		let root = world
			.spawn((
				BlobStore::new(FsStore::new(site_dir.clone())),
				default_router(),
				children![RoutesDir::new("routes")],
			))
			.flush();
		let watcher = world
			.spawn((LiveReload::new(site_dir.clone()), ClientIo))
			.flush();
		(root, watcher)
	}

	#[beet_core::test]
	async fn respawns_routes_and_reregisters_templates() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let site_dir = site_fixture("respawns");
		let (root, watcher) = spawn_site(&mut world, &site_dir);
		// the RoutesDir scan is async, so settle it before reading the tree
		AsyncRunner::settle_async_tasks(&mut world).await;
		let routes_dir =
			world.with_state::<Query<Entity, With<RoutesDir>>, _>(|query| {
				query.single().unwrap()
			});
		world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["docs"])
			.xpect_none();

		// mutate the site: a new route, an edited template, a new template
		fs_ext::write(site_dir.join("routes/docs/intro.md"), "# Intro")
			.unwrap();
		fs_ext::write(
			site_dir.join("templates/Card.bsx"),
			"<section>second card</section>",
		)
		.unwrap();
		fs_ext::write(site_dir.join("templates/Hero.bsx"), "<h1>hero</h1>")
			.unwrap();
		// drive the change directly instead of awaiting the debounced watcher
		world
			.entity_mut(watcher)
			.trigger_target(DirEvent::default());
		world.flush();
		// the respawn re-scans each RoutesDir asynchronously, so settle again
		AsyncRunner::settle_async_tasks(&mut world).await;

		// the new route landed in the rebuilt tree
		world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["docs", "intro"])
			.xpect_some();
		// the old routes respawned exactly once
		world
			.entity(routes_dir)
			.get::<Children>()
			.unwrap()
			.len()
			.xpect_eq(2);
		// the registry serves the edited and the new template sources
		let registry = world.resource::<BsxTemplateRegistry>();
		registry.contains("Hero").xpect_true();
		registry
			.get("Card")
			.unwrap()
			.nodes
			.xref()
			.xmap(|nodes| format!("{nodes:?}"))
			.xpect_contains("second card");
	}

	#[beet_core::test]
	fn broadcasts_reload_to_clients() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let site_dir = site_fixture("broadcasts");
		let (_root, watcher) = spawn_site(&mut world, &site_dir);
		let channel =
			world.with_state::<Query<Entity, With<ClientIo>>, _>(|query| {
				query.single().unwrap()
			});
		let received = Store::<Vec<Message>>::default();
		let captor = received.clone();
		world.spawn(ChildOf(channel)).observe_any(
			move |ev: On<MessageSend>| {
				captor.push(ev.event().inner().clone());
			},
		);

		world
			.entity_mut(watcher)
			.trigger_target(DirEvent::default());
		world.flush();

		received.get().xpect_eq(vec![Message::text(RELOAD_MESSAGE)]);
	}

	#[beet_core::test]
	fn spawns_a_channel_when_none_exists() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let site_dir = site_fixture("spawns_channel");
		// no pre-set ClientIo: the watcher spawns one as its child
		world
			.register_bsx_templates(site_dir.join("templates"))
			.unwrap();
		let watcher = world.spawn(LiveReload::new(site_dir.clone())).flush();
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
			.xpect_eq(watcher);
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
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let site_dir = deck_fixture("deck_marker");
		// the reload's render diagnostics paint the layout chrome, which reads
		// `PackageConfig`; seed it so the bare render world has it (see `site_layout`).
		world.init_resource::<PackageConfig>();
		// a deck router: the CardDeck marker (declared in the deck's markup spread);
		// the site store on the root backs the `RoutesDir` scan by ancestry.
		let router = world
			.spawn((
				BlobStore::new(FsStore::new(site_dir.clone())),
				Router,
				CardDeck,
				children![RoutesDir::new("slides")],
			))
			.flush();
		// the RoutesDir scan is async, so settle it before reading the tree
		AsyncRunner::settle_async_tasks(&mut world).await;
		card_order(&mut world, router)
			.xpect_eq(vec!["01-alpha".to_string(), "02-beta".to_string()]);

		// a new card, then a live reload (the watcher's change path).
		fs_ext::write(site_dir.join("slides/03-gamma.md"), "# Gamma").unwrap();
		reload_site(&mut world, &LiveReload::new(site_dir.clone())).unwrap();
		// the respawn re-scans each RoutesDir asynchronously, so settle again
		AsyncRunner::settle_async_tasks(&mut world).await;

		// the marker survived the route respawn ...
		world.entity(router).contains::<CardDeck>().xpect_true();
		// ... and the rebuilt tree still lists the cards in sorted order.
		card_order(&mut world, router).xpect_eq(vec![
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
				BlobStore::new(FsStore::new(site_dir.clone())),
				Router,
				CardDeck,
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

		// edit the current card on disk, then drive the watched-change reload.
		fs_ext::write(site_dir.join("slides/01-alpha.md"), "# Alpha edited")
			.unwrap();
		let site = LiveReload::new(site_dir.clone());
		app.world_mut()
			.commands()
			.queue(move |world: &mut World| reload_site(world, &site).unwrap());
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

	/// A deck's in-world navigator opens the first card on boot even when the
	/// cards are still being discovered: `CardStackPlugin`'s `open_initial_card`
	/// polls the route tree until a card resolves, rather than resolving once
	/// before the async `RoutesDir` scan lands and stranding the navigator on the
	/// generic home (which a deck has no route for). The navigator boots at `/`
	/// here *without* a pre-settle, the race the retry exists to absorb.
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	async fn deck_opens_first_card_despite_async_discovery() {
		use bevy::math::UVec2;

		let mut app = tui_app();
		// the stack-of-cards machinery contributes `open_initial_card`, the boot
		// patch under test (the serve binary adds it via `CardStackPlugin`).
		app.add_plugins(CardStackPlugin);
		let site_dir = deck_fixture("deck_boot");
		// the site store on the root backs the `RoutesDir` scan by ancestry.
		let router = app
			.world_mut()
			.spawn((
				BlobStore::new(FsStore::new(site_dir.clone())),
				Router,
				CardDeck,
				children![RoutesDir::new("slides")],
			))
			.flush();
		// deliberately no `settle_async_tasks` here: spawn the navigator at the
		// generic home while the RoutesDir scan is still in flight.
		let host = app
			.world_mut()
			.spawn((
				page_host(UVec2::new(40, 8)),
				Navigator::in_world(router, "/"),
			))
			.id();
		// `/` has no route on a deck, so the only way "Alpha first" paints is the
		// retry resolving the first card once discovery lands.
		drive_until(&mut app, host, "Alpha first").await;
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
				BlobStore::new(FsStore::new(site_dir.clone())),
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
