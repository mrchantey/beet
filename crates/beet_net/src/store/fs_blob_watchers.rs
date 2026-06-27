use crate::prelude::*;
use beet_core::prelude::*;

/// Registers a single directory to be watched for live reload, keyed to its owning
/// [`BlobStore`].
///
/// Each mount that exposes a directory (a `RoutesDir`'s scoped store, an
/// `AssetsDir`/`ServeBlobs` [`DirPath`], a `TemplateDir`, the entry document's own
/// dir) inserts one, so only those subtrees are watched, never the whole store root
/// (a recursive watch over the workspace exceeds the inotify limit and dies silently).
///
/// `dir` is the precise directory to watch (a store's `effective_root`); `store` is
/// the owning store, so emitted [`BlobEvent`]s are keyed to ITS base path
/// ([`root_key`](BlobStoreProvider::root_key)) and base-relative, routing through
/// the root live reload via [`did_change`](BlobStoreProvider::did_change).
/// Cross-platform; the native watcher observers back it (inert on wasm, where there
/// is no fs watcher).
#[derive(Debug, Clone, Component)]
pub struct WatchDir {
	/// The precise absolute directory to watch.
	pub dir: AbsPathBuf,
	/// The store the watched dir belongs to, keying emitted events to its base.
	pub store: BlobStore,
}

impl WatchDir {
	/// Watch `store`'s [`watch_dir`](BlobStoreProvider::watch_dir) (its
	/// `effective_root`), keyed to `store`. `None` for a store with no watchable
	/// directory (memory, S3), so a non-fs mount registers nothing.
	pub fn from_store(store: &BlobStore) -> Option<Self> {
		store.watch_dir().map(|dir| Self {
			dir,
			store: store.clone(),
		})
	}

	/// Watch the directory containing the entry document `entry_name` within
	/// `store`. `None` for a store with no watchable directory; a bare file-name
	/// entry resolves to the store's base dir.
	pub fn for_entry(store: &BlobStore, entry_name: &str) -> Option<Self> {
		let base = store.watch_dir()?;
		let dir = base.join(entry_name).parent().unwrap_or(base);
		Some(Self {
			dir,
			store: store.clone(),
		})
	}
}

// the native notify-based watcher backing `WatchDir`, requiring the `fs` feature
// (the `notify`/`FsWatcher` backend). Native-only: deno directory watching is
// unimplemented, so a wasm `FsStore` serves reads through `fs_ext` with no live
// reload; the `WatchDir` component above is inert there (and on a `std`-but-no-`fs`
// render target).
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub use native::*;

#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
mod native {
	use crate::prelude::*;
	use beet_core::exports::notify::EventKind;
	use beet_core::prelude::*;

	/// Native filesystem watchers backing reactive [`WatchDir`] mounts.
	///
	/// One [`FsWatcher`] entity per watched `dir`, refcounted by the [`WatchDir`]s
	/// sharing that dir (so a re-resolve that re-registers the same dir reuses its
	/// watcher). Each watcher emits events **relative to its store's base path**
	/// into the [`BlobEventBus`], so a subdir watcher's events still route through
	/// the base-keyed [`did_change`](BlobStoreProvider::did_change).
	#[derive(Default, Resource)]
	pub struct FsBlobWatchers {
		/// Per watched dir: the watcher entity and its subscriber refcount.
		watchers: HashMap<AbsPathBuf, FsWatcherEntry>,
	}

	/// A single watched-dir watcher and its refcount.
	struct FsWatcherEntry {
		/// The internal [`FsWatcher`] entity, despawned at zero refcount.
		entity: Entity,
		/// Number of [`WatchDir`]s sharing this dir.
		count: usize,
	}

	/// Ensure the added [`WatchDir`]'s dir is covered by exactly one watcher,
	/// bumping the refcount if it already exists.
	pub fn add_watch_dir(
		ev: On<Add, WatchDir>,
		mut commands: Commands,
		mut watchers: ResMut<FsBlobWatchers>,
		watch_dirs: Query<&WatchDir>,
	) {
		let Ok(watch) = watch_dirs.get(ev.entity) else {
			return;
		};
		let dir = watch.dir.clone();
		if let Some(entry) = watchers.watchers.get_mut(&dir) {
			entry.count += 1;
			return;
		}
		// the dir may not exist yet (eg an `assets/` an entry declares but has not
		// created), but an `FsWatcher` cannot watch a path that does not exist.
		fs_ext::create_dir_all(&dir).ok();
		// spawn an internal FsWatcher on the dir (the cargo-project filter excludes
		// target/.git/.beet/codegen/rustc-ice churn), forwarding DirEvents to the bus
		// relative to (and keyed to) the store's base, so they route via `did_change`.
		let base = watch.store.base_dir();
		let store = watch.store.clone();
		let entity = commands
			.spawn(FsWatcher::default_cargo().with_path(dir.clone()))
			.observe_any(move |ev: On<DirEvent>, bus: Res<BlobEventBus>| {
				forward_dir_event_for(&ev, base.as_ref(), &store, &bus);
			})
			.id();
		watchers
			.watchers
			.insert(dir, FsWatcherEntry { entity, count: 1 });
	}

	/// Drop the watcher refcount for the removed [`WatchDir`], despawning the
	/// watcher entity at zero.
	pub fn remove_watch_dir(
		ev: On<Remove, WatchDir>,
		mut commands: Commands,
		mut watchers: ResMut<FsBlobWatchers>,
		watch_dirs: Query<&WatchDir>,
	) {
		let Ok(watch) = watch_dirs.get(ev.entity) else {
			return;
		};
		let dir = watch.dir.clone();
		if let Some(entry) = watchers.watchers.get_mut(&dir) {
			entry.count -= 1;
			if entry.count == 0 {
				commands.entity(entry.entity).despawn();
				watchers.watchers.remove(&dir);
			}
		}
	}

	impl FsBlobWatchers {
		/// Number of distinct watched-dir watchers, for tests.
		pub fn len(&self) -> usize { self.watchers.len() }

		/// Whether there are no active watchers.
		pub fn is_empty(&self) -> bool { self.watchers.is_empty() }
	}

	/// Convert a [`DirEvent`] into per-object [`BlobEvent`]s for `store`, each path
	/// stripped to `base` (so it is base-relative) and sent keyed to `store` (so
	/// `store.root_key()` is the base, routing via `did_change`). A `None` `base`
	/// or a path outside it is skipped.
	pub fn forward_dir_event_for(
		event: &DirEvent,
		base: Option<&AbsPathBuf>,
		store: &BlobStore,
		bus: &BlobEventBus,
	) {
		let Some(base) = base else { return };
		event.iter().for_each(|path_event| {
			// base-relative path, skipping events outside the base
			let Ok(rel) = path_event.path.strip_prefix(base.as_ref()) else {
				return;
			};
			// directories are not objects
			if path_event.path.is_dir() {
				return;
			}
			let kind = match path_event.kind {
				EventKind::Create(_) => BlobEventKind::Created,
				EventKind::Remove(_) => BlobEventKind::Removed,
				_ => BlobEventKind::Changed,
			};
			bus.send(BlobEvent::new(
				store.clone(),
				SmolPath::from(rel),
				kind,
			));
		});
	}

	#[cfg(test)]
	mod test {
		use crate::prelude::*;
		use beet_core::prelude::*;
		use std::sync::Arc;
		use std::sync::atomic::AtomicBool;
		use std::sync::atomic::Ordering;

		/// Two [`WatchDir`]s over the same dir dedup to one refcounted watcher.
		#[beet_core::test]
		fn dedups_by_watched_dir() {
			let mut world =
				(MinimalPlugins, AsyncPlugin, StorePlugin).into_world();
			let path = AbsPathBuf::new_workspace_rel(
				"target/tests/beet_net/fs_blob_watchers",
			)
			.unwrap();
			let store = BlobStore::new(FsStore::new(path.clone()));

			// same watched dir, two registrations
			let watch = WatchDir::from_store(&store).unwrap();
			let a = world.spawn(watch.clone()).id();
			let b = world.spawn(watch).id();
			world.flush();
			world.resource::<FsBlobWatchers>().len().xpect_eq(1);

			// despawning one leaves the shared watcher alive
			world.entity_mut(a).despawn();
			world.flush();
			world.resource::<FsBlobWatchers>().len().xpect_eq(1);

			// despawning the last drops it
			world.entity_mut(b).despawn();
			world.flush();
			world.resource::<FsBlobWatchers>().is_empty().xpect_true();
		}

		/// A [`WatchDir`] over a subdir emits a BASE-relative, base-keyed
		/// [`BlobEvent`] when a file in the subdir changes, so it routes through the
		/// base store's [`did_change`](BlobStoreProvider::did_change).
		#[beet_core::test]
		async fn subdir_watcher_emits_base_relative_event() {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, AsyncPlugin, StorePlugin));
			// a clean base dir with a `sub/` subdir the watcher covers. Under the
			// system temp dir, NOT `target/`: the watcher's `default_cargo` filter
			// excludes `*target*`, so a `target/`-rooted fixture's events never fire.
			let base = AbsPathBuf::new(
				std::env::temp_dir().join("beet_net_watch_subdir"),
			)
			.unwrap();
			fs_ext::remove(&base).ok();
			fs_ext::create_dir_all(base.join("sub")).unwrap();
			// the base store keys events; the watcher covers only its `sub/` subdir
			let store = BlobStore::new(FsStore::new(base.clone()));
			let scoped = store.with_subdir(SmolPath::from("sub"));
			app.world_mut().spawn(WatchDir::from_store(&scoped).unwrap());

			// capture the first matching event and exit; the run loop (not a manual
			// `update` loop) drives the watcher's async task + drains the bus.
			let captured = Store::<Vec<BlobEvent>>::default();
			let captor = captured.clone();
			app.world_mut().add_observer(
				move |ev: On<BlobEvent>, mut commands: Commands| {
					captor.push(ev.clone());
					commands.write_message(AppExit::Success);
				},
			);

			// De-flake: the watcher may not be registered when the first write lands,
			// and the debouncer batches, so keep writing distinct content (off-thread,
			// as in `fs_watcher.rs`'s `works`) until the observer triggers exit.
			let done = Arc::new(AtomicBool::new(false));
			let done2 = done.clone();
			let write_dir = base.join("sub");
			std::thread::spawn(move || {
				let mut i = 0u32;
				while !done2.load(Ordering::Relaxed) {
					std::thread::sleep(Duration::from_millis(50));
					let _ = fs_ext::write(
						write_dir.join("style.css"),
						format!("body {{}} /* {i} */"),
					);
					i += 1;
				}
			});
			app.run_async().await.xpect_eq(AppExit::Success);
			done.store(true, Ordering::Relaxed);

			let captured = captured.get();
			let event = captured.first().unwrap();
			// base-relative path (NOT watcher-relative): `sub/style.css`
			event.path.to_string().xpect_eq("sub/style.css");
			// keyed to the base store, so `did_change` routes it from the root
			event.store.root_key().xpect_eq(store.root_key());
			store.did_change(event).xpect_true();
		}
	}
}
