//! Dev-mode live reload: watch the site dir, refresh the world, tell clients.

use crate::prelude::*;
use beet_core::prelude::*;
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
	// respawn each RoutesDir's routes: the Insert observer re-scans the dir
	// and the respawned PathPatterns rebuild the route tree
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
			.insert(dir);
	}
	world.flush();
	// tell connected clients to reload
	let channels = world.with_state::<Query<Entity, With<ClientIo>>, _>(
		|query| query.iter().collect::<Vec<_>>(),
	);
	for channel in channels {
		world
			.entity_mut(channel)
			.trigger_target(ClientIoBroadcast(Message::text(RELOAD_MESSAGE)));
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use super::*;

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
	fn spawn_site(world: &mut World, site_dir: &AbsPathBuf) -> (Entity, Entity) {
		world.register_bsx_templates(site_dir.join("templates")).unwrap();
		world.insert_resource(SiteRoot(site_dir.clone()));
		let root = world
			.spawn((default_router(), children![RoutesDir::new("routes")]))
			.flush();
		let watcher = world
			.spawn((LiveReload::new(site_dir.clone()), ClientIo))
			.flush();
		(root, watcher)
	}

	#[beet_core::test]
	fn respawns_routes_and_reregisters_templates() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let site_dir = site_fixture("respawns");
		let (root, watcher) = spawn_site(&mut world, &site_dir);
		let routes_dir = world
			.with_state::<Query<Entity, With<RoutesDir>>, _>(|query| {
				query.single().unwrap()
			});
		world
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
		// drive the change directly instead of awaiting the debounced watcher
		world.entity_mut(watcher).trigger_target(DirEvent::default());
		world.flush();

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
		let channel = world
			.with_state::<Query<Entity, With<ClientIo>>, _>(|query| {
				query.single().unwrap()
			});
		let received = Store::<Vec<Message>>::default();
		let captor = received.clone();
		world
			.spawn(ChildOf(channel))
			.observe_any(move |ev: On<MessageSend>| {
				captor.push(ev.event().inner().clone());
			});

		world.entity_mut(watcher).trigger_target(DirEvent::default());
		world.flush();

		received.get().xpect_eq(vec![Message::text(RELOAD_MESSAGE)]);
	}

	#[beet_core::test]
	fn spawns_a_channel_when_none_exists() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let site_dir = site_fixture("spawns_channel");
		// no pre-set ClientIo: the watcher spawns one as its child
		world.register_bsx_templates(site_dir.join("templates")).unwrap();
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
}
