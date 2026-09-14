use crate::prelude::*;
use beet_core::prelude::*;

/// Registers a single directory to be watched for live reload, keyed to the
/// [`base`](BlobStore::base) of its owning [`BlobStore`].
///
/// Each mount that exposes a directory (a `RoutesDir`/`TemplateDir`/`AssetsDir`
/// [`DirPath`] scope, the entry document's own dir) carries one, so only those
/// subtrees are watched, never the whole store root (a recursive watch over the
/// workspace exceeds the inotify limit and dies silently).
///
/// `dir` is the precise directory to watch (a store's `effective_root`); `base`
/// is the owning store's unscoped root, so emitted [`BlobEvent`]s are keyed to
/// it and base-relative, routing through the root live reload via
/// [`did_change`](BlobStoreProvider::did_change). The live set of these derives
/// the watcher set (see [`FsBlobWatchers`]). Cross-platform; the native watcher
/// observers back it (inert on wasm, where there is no fs watcher).
#[derive(Debug, Clone, Component)]
pub struct WatchDir {
	/// The precise absolute directory to watch.
	pub dir: AbsPath,
	/// The unscoped store at the watched store's root, keying emitted events.
	pub base: BlobStore,
}

impl WatchDir {
	/// Watch `store`'s [`watch_dir`](BlobStoreProvider::watch_dir) (its
	/// `effective_root`), keyed to its base. `None` for a store with no
	/// watchable directory (memory, S3), so a non-fs mount registers nothing.
	pub fn from_store(store: &BlobStore) -> Option<Self> {
		store.watch_dir().map(|dir| Self {
			dir,
			base: store.base(),
		})
	}

	/// Watch the directory containing the entry document `entry_name` within
	/// `store`. `None` for a store with no watchable directory; a bare file-name
	/// entry resolves to the store's watch dir.
	pub fn for_entry(store: &BlobStore, entry_name: &str) -> Option<Self> {
		let root = store.watch_dir()?;
		let dir = root.join(entry_name).parent().unwrap_or(root);
		Some(Self {
			dir,
			base: store.base(),
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
	/// The watcher set is derived from the live registrations: one recursive
	/// [`FsWatcher`] per *minimal root*, a registered dir with no registered
	/// strict ancestor keyed to the same base, so a `routes/` under a watched
	/// entry dir rides the entry's watcher and one edit never surfaces twice.
	/// Every registration change re-derives the set and diffs it against the
	/// live watchers ([`sync`](Self::sync)); nothing is refcounted. A watcher
	/// emits events keyed to its base and base-relative, so whichever watcher
	/// saw the change, it routes identically through
	/// [`did_change`](BlobStoreProvider::did_change) and [`Blob::matches_event`].
	#[derive(Default, Resource)]
	pub struct FsBlobWatchers {
		/// Per live [`WatchDir`] entity: the root it registers and its base store.
		registrations: HashMap<Entity, (WatchRoot, BlobStore)>,
		/// Per minimal root: the [`FsWatcher`] entity observing it.
		roots: HashMap<WatchRoot, Entity>,
	}

	/// A watched directory and the base dir its events are stripped to. Two
	/// stores with different bases over one dir are two roots: each keys its
	/// events to its own base.
	#[derive(Debug, Clone, PartialEq, Eq, Hash)]
	struct WatchRoot {
		base: AbsPath,
		dir: AbsPath,
	}

	/// Register the inserted [`WatchDir`] (a replaced one re-registers under the
	/// same entity) and re-derive the watcher set.
	pub fn add_watch_dir(
		ev: On<Insert, WatchDir>,
		mut commands: Commands,
		mut watchers: ResMut<FsBlobWatchers>,
		watch_dirs: Query<&WatchDir>,
	) {
		let Ok(watch) = watch_dirs.get(ev.entity) else {
			return;
		};
		// a base with no local directory (memory, S3) has nothing to watch
		let Some(base) = watch.base.base_dir() else {
			return;
		};
		let root = WatchRoot {
			base,
			dir: watch.dir.clone(),
		};
		watchers
			.registrations
			.insert(ev.entity, (root, watch.base.clone()));
		watchers.sync(&mut commands);
	}

	/// Drop the removed [`WatchDir`]'s registration and re-derive the watcher
	/// set, despawning any watcher no registration needs.
	pub fn remove_watch_dir(
		ev: On<Remove, WatchDir>,
		mut commands: Commands,
		mut watchers: ResMut<FsBlobWatchers>,
	) {
		if watchers.registrations.remove(&ev.entity).is_some() {
			watchers.sync(&mut commands);
		}
	}

	impl FsBlobWatchers {
		/// The dirs currently watched, one per minimal root.
		pub fn dirs(&self) -> impl Iterator<Item = &AbsPath> {
			self.roots.keys().map(|root| &root.dir)
		}

		/// The minimal root set: every registered root with no registered strict
		/// ancestor keyed to the same base, each with its base store.
		fn minimal_roots(&self) -> HashMap<WatchRoot, BlobStore> {
			self.registrations
				.values()
				.filter(|(root, _)| {
					!self.registrations.values().any(|(other, _)| {
						other.base == root.base
							&& other.dir != root.dir
							&& root.dir.strip_prefix(&other.dir).is_some()
					})
				})
				.map(|(root, base)| (root.clone(), base.clone()))
				.collect()
		}

		/// Diff the minimal root set against the live watchers: despawn each
		/// watcher no root needs, spawn one for each root without.
		fn sync(&mut self, commands: &mut Commands) {
			let wanted = self.minimal_roots();
			for (_, entity) in
				self.roots.extract_if(|root, _| !wanted.contains_key(root))
			{
				commands.entity(entity).despawn();
			}
			for (root, base) in wanted {
				if !self.roots.contains_key(&root) {
					let entity = Self::spawn_watcher(commands, &root, base);
					self.roots.insert(root, entity);
				}
			}
		}

		/// Spawn an internal [`FsWatcher`] on `root`'s dir (the cargo-project
		/// filter excludes target/.git/.beet/codegen/rustc-ice churn), forwarding
		/// its [`DirEvent`]s to the bus relative to (and keyed to) `base`, so they
		/// route via [`did_change`](BlobStoreProvider::did_change).
		fn spawn_watcher(
			commands: &mut Commands,
			root: &WatchRoot,
			base: BlobStore,
		) -> Entity {
			// the dir may not exist yet (eg an `assets/` an entry declares but has
			// not created), but an `FsWatcher` cannot watch a path that does not
			// exist.
			fs_ext::create_dir_all(&root.dir).ok();
			let base_dir = root.base.clone();
			commands
				.spawn(FsWatcher::default_cargo().with_path(root.dir.clone()))
				.observe_any(move |ev: On<DirEvent>, bus: Res<BlobEventBus>| {
					forward_dir_event_for(&ev, &base_dir, &base, &bus);
				})
				.id()
		}
	}

	/// Convert a [`DirEvent`] into per-object [`BlobEvent`]s keyed to `base`
	/// (the unscoped store at `base_dir`), each path stripped to `base_dir` so
	/// the event's `path` and `root_relative_path` agree. A path outside the base
	/// is skipped.
	fn forward_dir_event_for(
		event: &DirEvent,
		base_dir: &AbsPath,
		base: &BlobStore,
		bus: &BlobEventBus,
	) {
		event.iter().for_each(|path_event| {
			// base-relative path, skipping events outside the base
			let Some(rel) = path_event.path.strip_prefix(base_dir) else {
				return;
			};
			// directories are not objects
			if fs_ext::is_dir(&path_event.path) {
				return;
			}
			let kind = match path_event.kind {
				EventKind::Create(_) => BlobEventKind::Created,
				EventKind::Remove(_) => BlobEventKind::Removed,
				_ => BlobEventKind::Changed,
			};
			bus.send(BlobEvent::new(base.clone(), rel, kind));
		});
	}

	#[cfg(test)]
	mod test {
		use crate::prelude::*;
		use beet_core::prelude::*;
		use std::sync::Arc;
		use std::sync::atomic::AtomicBool;
		use std::sync::atomic::Ordering;

		fn watcher_world() -> World {
			(MinimalPlugins, AsyncPlugin, StorePlugin).into_world()
		}

		/// An fs store under `target/tests`.
		fn fs_store(name: &str) -> BlobStore {
			AbsPath::new_workspace_rel(format!("target/tests/beet_net/{name}"))
				.unwrap()
				.xmap(FsStore::new)
				.xmap(BlobStore::new)
		}

		/// The watched dirs, sorted for comparison.
		fn watched(world: &World) -> Vec<String> {
			world
				.resource::<FsBlobWatchers>()
				.dirs()
				.map(|dir| dir.to_string())
				.collect::<Vec<_>>()
				.xtap(|dirs| dirs.sort())
		}

		/// The dir a store's [`WatchDir`] names.
		fn dir_of(store: &BlobStore) -> String {
			store.watch_dir().unwrap().to_string()
		}

		/// Spawn a [`WatchDir`] over `store` and flush.
		fn watch(world: &mut World, store: &BlobStore) -> Entity {
			world.spawn(WatchDir::from_store(store).unwrap()).flush()
		}

		/// Two [`WatchDir`]s over the same dir derive one watcher, which stays
		/// while either registration remains.
		#[beet_core::test]
		fn dedups_by_watched_dir() {
			let mut world = watcher_world();
			let store = fs_store("fs_blob_watchers");
			let first = watch(&mut world, &store);
			let second = watch(&mut world, &store);
			world
				.resource::<FsBlobWatchers>()
				.dirs()
				.count()
				.xpect_eq(1);

			world.entity_mut(first).despawn();
			world.flush();
			world
				.resource::<FsBlobWatchers>()
				.dirs()
				.count()
				.xpect_eq(1);

			world.entity_mut(second).despawn();
			world.flush();
			world
				.resource::<FsBlobWatchers>()
				.dirs()
				.count()
				.xpect_eq(0);
		}

		/// The watcher set is the minimal root set of the registrations, whatever
		/// order they arrive or leave in: an ancestor absorbs the dirs under it,
		/// and its removal hands them back their own watchers.
		#[beet_core::test]
		fn derives_minimal_roots() {
			let mut world = watcher_world();
			let store = fs_store("fs_blob_watchers_roots");
			let routes = store.with_subdir(RelPath::from("routes"));
			let templates = store.with_subdir(RelPath::from("templates"));
			let blog = store.with_subdir(RelPath::from("routes/blog"));

			// descendants first: two roots ...
			let routes_watch = watch(&mut world, &routes);
			let templates_watch = watch(&mut world, &templates);
			watched(&world).xpect_eq(vec![dir_of(&routes), dir_of(&templates)]);
			// ... collapsed by their ancestor, which a further descendant rides
			let root_watch = watch(&mut world, &store);
			let blog_watch = watch(&mut world, &blog);
			watched(&world).xpect_eq(vec![dir_of(&store)]);

			// removing the ancestor re-derives the roots the rest still need
			world.entity_mut(root_watch).despawn();
			world.flush();
			watched(&world).xpect_eq(vec![dir_of(&routes), dir_of(&templates)]);
			world.entity_mut(routes_watch).despawn();
			world.flush();
			watched(&world).xpect_eq(vec![dir_of(&blog), dir_of(&templates)]);
			world.entity_mut(templates_watch).despawn();
			world.entity_mut(blog_watch).despawn();
			world.flush();
			watched(&world).xpect_eq(Vec::<String>::new());
		}

		/// Two stores with different bases over one dir are two roots: an
		/// ancestor watcher keyed to another base cannot serve a store's events.
		#[beet_core::test]
		fn roots_are_per_base() {
			let mut world = watcher_world();
			let outer = fs_store("fs_blob_watchers_bases");
			let scoped = outer.with_subdir(RelPath::from("inner"));
			let inner = fs_store("fs_blob_watchers_bases/inner");
			watch(&mut world, &outer);
			watch(&mut world, &scoped);
			watch(&mut world, &inner);
			watched(&world).xpect_eq(vec![dir_of(&outer), dir_of(&inner)]);
		}

		/// A [`WatchDir`] over a subdir emits a BASE-relative, base-keyed
		/// [`BlobEvent`] when a file in the subdir changes, so it routes through the
		/// base store's [`did_change`](BlobStoreProvider::did_change) and matches
		/// a [`Blob`] of the scoped store.
		#[beet_core::test]
		async fn subdir_watcher_emits_base_relative_event() {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, AsyncPlugin, StorePlugin));
			// a clean base dir with a `sub/` subdir the watcher covers
			let base = AbsPath::new(
				std::env::temp_dir().join("beet_net_watch_subdir"),
			)
			.unwrap();
			fs_ext::remove(&base).ok();
			fs_ext::create_dir_all(base.join("sub")).unwrap();
			// the base store keys events; the watcher covers only its `sub/` subdir
			let store = BlobStore::new(FsStore::new(base.clone()));
			let scoped = store.with_subdir(RelPath::from("sub"));
			app.world_mut()
				.spawn(WatchDir::from_store(&scoped).unwrap());

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
			// keyed to the base store (the scoped store's `base()` agrees with a
			// fresh one), so `did_change` routes it from the root and the scoped
			// store's blob recognizes its own object
			event.store.same_scope(&store).xpect_true();
			event
				.root_relative_path()
				.to_string()
				.xpect_eq("sub/style.css");
			store.did_change(event).xpect_true();
			scoped
				.blob(RelPath::from("style.css"))
				.matches_event(event)
				.xpect_true();
		}
	}
}
