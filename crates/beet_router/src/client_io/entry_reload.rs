//! Full entry rebuild on a structural change, the heavy half of [live
//! reload](super::LiveReload).
//!
//! A markdown or template edit is a *content* change: the light re-fire in
//! [`reload_site`](super::reload_site) re-registers the templates and respawns the
//! routes in place, leaving the servers and their sockets up. A change to the entry
//! document itself or an included `<Template src>` is *structural* (a route added,
//! a server reconfigured, a capability rewired), which the in-place re-fire cannot
//! express. Such a change instead drives a full teardown+rebuild: the whole entry
//! scene is despawned (servers close, sockets drop) and rebuilt from the current
//! source (servers rebind, the browser reconnects and reloads), leaving no entity
//! behind.
//!
//! The entry-build logic lives above `beet_router` (in the entry driver, ie the
//! `beet` cli), so it is carried down as a boxed callback on the [`EntryReloader`]
//! resource, which the driver installs on `--watch`. The reload path here only
//! decides *when* a rebuild is due and invokes the callback.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::sockets::Message;
use std::sync::Arc;

/// The driver's entry-rebuild callback: tear down the old entry scene and build a
/// fresh one from the current source, given the async world.
///
/// Boxed so the driver's build logic rides down into the reload path; the future
/// is local (`!Send`) since it drives the entry build on the local pool, matching
/// how the initial entry load runs.
type EntryRebuildFn =
	Arc<dyn Fn(AsyncWorld) -> LocalBoxedFuture<'static, Result> + Send + Sync>;

/// Installed by the entry driver (the `beet` cli) on `--watch` so a change to a
/// *structural* source drives a full teardown+rebuild rather than the light content
/// re-fire a markdown/template edit gets.
///
/// Carries the driver's [rebuild callback](EntryRebuildFn) plus the set of source
/// paths whose change is structural: the entry document and every transitive
/// `<Template src>` include, each store-root-relative so it matches the
/// [`BlobEvent`] paths the watcher emits.
#[derive(Clone, Resource)]
pub struct EntryReloader {
	sources: HashSet<SmolPath>,
	rebuild: EntryRebuildFn,
}

impl EntryReloader {
	/// A reloader over the given structural `sources`, rebuilding through `rebuild`.
	pub fn new(
		sources: HashSet<SmolPath>,
		rebuild: impl 'static
		+ Send
		+ Sync
		+ Fn(AsyncWorld) -> LocalBoxedFuture<'static, Result>,
	) -> Self {
		Self {
			sources,
			rebuild: Arc::new(rebuild),
		}
	}

	/// Whether a change to `path` is structural (the entry document or a
	/// `<Template src>` include), so it drives a full rebuild rather than the light
	/// content re-fire.
	pub fn is_structural(&self, path: &SmolPath) -> bool {
		self.sources.contains(path)
	}
}

/// Marks a [`LiveReload`](super::LiveReload) site whose pending reload is a full
/// entry rebuild (a structural source changed), rather than the light content
/// re-fire [`NeedsReload`](super::NeedsReload) alone drives. Always set alongside
/// `NeedsReload`, so the shared reload gate and debounce still apply.
#[derive(Component)]
pub(crate) struct NeedsEntryRebuild;

/// Full teardown+rebuild of the entry scene: hand off to the driver's
/// [`EntryReloader`], which tears down the old [`BeetSceneRoot`] via `despawn_scene`
/// (servers close, sockets drop) and builds a fresh entry root (servers rebind, the
/// browser reconnects and reloads via [`live_reload.js`]). Degrades to the light
/// [`reload_site`](super::reload_site) when no driver is installed (eg an in-process
/// test with no cli).
///
/// The old `root` (a [`LiveReload`](super::LiveReload) site) is despawned by the
/// teardown, so its `Reloading` guard rides the despawn and the fresh root carries
/// its own `LiveReload`/`WatchDir` for the next edit.
pub(crate) fn rebuild_entry(world: &mut World, root: Entity) {
	let Some(reloader) = world.get_resource::<EntryReloader>().cloned() else {
		warn!(
			"structural change with no entry reloader installed; re-firing content"
		);
		reload_site(world, root);
		return;
	};
	// close the live client sockets so the browser sees the teardown and reconnects
	// to the rebuilt server, reloading on reconnect. The socket's reader/writer tasks
	// outlive the entity despawn (an entity-scoped task ends on its own, not on
	// despawn), so despawning alone leaves the connection open; an explicit close is
	// what the browser observes.
	disconnect_clients(world);
	// drive the rebuild on the local pool (as the initial load runs), so the
	// despawn+respawn and the fresh root's `BootOnLoad` boot land on the runtime.
	world.run_async_local(move |world| async move {
		if let Err(err) = (reloader.rebuild)(world).await {
			error!("entry rebuild failed: {err}");
		}
		Ok(())
	});
}

/// Send a websocket close to every [`ClientIo`] client, so a browser detects the
/// teardown and reconnects to the rebuilt server (reloading on reconnect via
/// [`live_reload.js`]). The close frame rides the writer channel, which the writer
/// task drains even after the socket entity is despawned, so the browser sees it
/// regardless of despawn ordering.
fn disconnect_clients(world: &mut World) {
	let channels = world
		.with_state::<Query<(Entity, Option<&Children>), With<ClientIo>>, _>(
			|query| {
				query
					.iter()
					.map(|(channel, clients)| {
						(channel, clients.map_or(0, |clients| clients.len()))
					})
					.collect::<Vec<_>>()
			},
		);
	let clients: usize = channels.iter().map(|(_, count)| count).sum();
	debug!("live reload: closing {clients} client socket(s) before teardown");
	for (channel, _) in channels {
		world
			.entity_mut(channel)
			.trigger_target(ClientIoBroadcast(Message::Close(None)));
	}
	// `broadcast_to_clients` queues the per-client `MessageSend` via `Commands`, so
	// flush to push the close into each writer channel before the async teardown.
	world.flush();
}

#[cfg(all(test, feature = "template_serde"))]
mod test {
	use super::*;
	use beet_net::prelude::*;

	/// Tear down the previous entry scene and spawn a fresh `BeetSceneRoot`
	/// [`LiveReload`](super::LiveReload) site over `store`, as the cli's
	/// `rebuild_watched_entry` does. Returns the new root.
	fn spawn_entry_root(world: &mut World, store: &InMemoryStore) -> Entity {
		despawn_scene(world);
		world
			.spawn((
				BlobStore::new(store.clone()),
				LiveReload::new(),
				BeetSceneRoot,
			))
			.flush()
	}

	/// A structural change routes through the driver's rebuild (`despawn_scene` then
	/// a fresh spawn), so across many reloads exactly one `BeetSceneRoot` and one
	/// `LiveReload` site survive: the teardown despawns the old root before the new
	/// one is built, so nothing accumulates.
	#[beet_core::test]
	async fn full_rebuild_leaves_no_leaked_roots() {
		let mut world =
			(MinimalPlugins, AsyncPlugin, RouterPlugin).into_world();
		let store = InMemoryStore::new();
		// the driver's rebuild callback: spawn a fresh entry root under a world lock.
		let rebuild_store = store.clone();
		world.insert_resource(EntryReloader::new(
			[SmolPath::from("main.bsx")].into_iter().collect(),
			move |world: AsyncWorld| {
				let store = rebuild_store.clone();
				Box::pin(async move {
					world
						.with(move |world: &mut World| {
							spawn_entry_root(world, &store);
						})
						.await;
					Ok(())
				})
			},
		));
		// the initial entry root, then several structural rebuilds through the real
		// reload path (mark as `reload_site_on_change` would, then process).
		spawn_entry_root(&mut world, &store);
		for _ in 0..5 {
			let root = world
				.with_state::<Query<Entity, With<LiveReload>>, _>(|query| {
					query.single().unwrap()
				});
			world.entity_mut(root).insert((NeedsReload, NeedsEntryRebuild));
			process_live_reloads(&mut world);
			AsyncRunner::settle_async_tasks(&mut world).await;
		}
		// exactly one of each survives: despawn-all-then-spawn-one never accumulates.
		world
			.with_state::<Query<(), With<BeetSceneRoot>>, _>(|query| {
				query.iter().count()
			})
			.xpect_eq(1);
		world
			.with_state::<Query<(), With<LiveReload>>, _>(|query| {
				query.iter().count()
			})
			.xpect_eq(1);
	}
}
