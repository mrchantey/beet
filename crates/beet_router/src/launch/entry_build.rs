//! The cross-platform entry resolution + build core: every binary that loads a
//! beet entry runs through here, as do the `check`/`serve`/`export-static`
//! commands and the wasm Worker entry.
//!
//! An entry load splits into resolution ([`resolve_entry_path`]: the store + the entry
//! document name within it, honouring `--repo` and the entry's own
//! `<RepoRoot src>`), a world-free async read ([`read_sources`]: the entry
//! document, through the [`BlobStore`]) and a synchronous world build
//! ([`build_root`]: lower the entry and build it into a root carrying the store
//! and the package its references resolve in). A document the entry
//! instantiates loads as the tree builds, holding the root's [`Ready`] until it
//! has. The same path runs on the native async runtime and the single-threaded
//! wasm Worker, so an entry build never requires a filesystem.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_scene::prelude::*;
use std::sync::Arc;

/// Entry-document file names discovery looks for, in priority order. The native
/// binary walks the cwd and its ancestors for the first match; the `check`/`serve`/
/// `export-static` commands search a single given site dir for it. Shared so both
/// agree on what an entry document is named.
pub const ENTRY_NAMES: &[&str] = &["main.bsx", "main.json"];

/// Pre-scan the raw entry document through `store` against the registered
/// [`Prescan`] set, the one registry-free walk entry resolution reads its
/// declarations from.
pub async fn read_prescan(
	repo_store: &BlobStore,
	prescans: &PrescanRegistry,
	entry_name: &str,
) -> Result<EntryPrescan> {
	let path = RelPath::from(entry_name);
	repo_store
		.get_media(&path)
		.await?
		.xmap(|entry| EntryPrescan::parse(&entry, Some(&path), prescans))
}

/// A resolved entry: its store, the entry document name within it, and the local
/// dir to watch for dev live reload (`None` for a store with no local root, ie
/// a self-rooted store, and always `None` on wasm, where there is no fs-watcher
/// backend).
#[derive(Debug)]
pub struct ResolvedEntry {
	pub repo_store: BlobStore,
	pub entry_name: String,
	/// The entry document's declarations, read by the same pass that resolved the
	/// store, so the load never re-scans it.
	pub prescan: EntryPrescan,
	#[cfg(not(target_arch = "wasm32"))]
	pub watch_dir: Option<AbsPath>,
}

/// Resolve an entry within its store: read the prescan once, rebase the
/// store through the entry's own `<RepoRoot src>` declaration
/// ([`BlobStore::rebase_repo`]) when it carries one, then run every other
/// registered declaration's preload against the rebased store
/// ([`PrescanRegistry::preload`]: the `<Secrets>` load into the process
/// environment, and whatever a downstream crate registered). The one
/// widening path every entry load shares (the binary, discovery,
/// `serve`/`check`/`export-static`, the Worker); callers differ only in how
/// the initial `(store, entry_name)` pair is derived (a local path walk vs a
/// key in a self-rooted store).
///
/// Live reload watches the store's local root when it has one
/// ([`BlobProvider::watch_dir`]) — the rebased root, so the watcher sees
/// the entry's whole declared universe; a store with no local directory
/// watches nothing.
pub async fn resolve_in_repo_store(
	repo_store: BlobStore,
	prescans: &PrescanRegistry,
	entry_name: String,
) -> Result<ResolvedEntry> {
	let prescan = read_prescan(&repo_store, prescans, &entry_name).await?;
	// the rebased store holds the same entry document, so its pre-scan is the
	// one already read: entry resolution scans the entry exactly once.
	let (repo_store, entry_name) = match prescan.first::<RepoRoot>() {
		Some(root) => repo_store.rebase_repo(&entry_name, &root.src)?,
		None => (repo_store, entry_name),
	};
	prescans.preload(&prescan, &repo_store).await?;
	Ok(ResolvedEntry {
		#[cfg(not(target_arch = "wasm32"))]
		watch_dir: repo_store.watch_dir(),
		repo_store,
		entry_name,
		prescan,
	})
}

/// Resolve the entry [`BlobStore`], the entry document name within it, and the
/// local directory to watch for dev live reload (`None` when the store has no
/// local root, ie a self-rooted store). The one launch resolution every
/// world-owning driver runs: the binary's `Startup` loader (native, and the
/// browser the served page boots) and the wasm Worker, each handing in the
/// `--repo`/`--store-fork`/`--entry` its process config carries.
///
/// Resolution order:
/// 1. a self-rooted `repo_uri` (`s3://<bucket>`, `r2://<binding>`,
///    `indexed-db://<db>`, `http:<prefix>`): the store roots itself, so `entry`
///    names the entry document *within* it, defaulting to an [`ENTRY_NAMES`]
///    probe. A deployed task passes `--repo=s3://<bucket>` (deploy config as
///    args, not env); a served page's bootstrap passes `--repo=http:repo`.
/// 2. `entry=<path>`: the entry file itself (a recognized extension) or a
///    directory probed for [`ENTRY_NAMES`]; see [`resolve_entry_path`].
/// 3. otherwise: discovery walks the cwd and its ancestors through an `fs`
///    store for the first [`ENTRY_NAMES`] match.
///
/// Every path then resolves through [`resolve_in_repo_store`], so an entry's
/// `<RepoRoot src>` declaration rebases any store kind uniformly (an fs store
/// re-roots, a self-rooted store takes a key-prefix view or fails loudly on a
/// mis-publish). The [`StoreUri`] selects the backend (default `fs`), and a
/// `store_fork` (else [`default_store_fork`]) forks it into a local store.
///
/// Target-agnostic: wasm runs the same walk wherever the runtime has a
/// filesystem (deno/node through the runner's fs globals); a fs-less runtime
/// (a browser tab) errors with guidance when handed a dir-rooted repo.
pub async fn resolve_entry(
	prescans: &PrescanRegistry,
	repo_uri: Option<&StoreUri>,
	store_fork: Option<&StoreUri>,
	entry: Option<&str>,
) -> Result<ResolvedEntry> {
	// a self-rooted store: no local dir and no ancestor walk, so `entry` is a
	// key within the store, defaulting to the entry-name probe.
	if let Some(uri) = repo_uri.filter(|uri| uri.is_self_rooted()) {
		let repo_store = compose_repo_store(uri, store_fork)?;
		let entry_name = self_rooted_entry_name(&repo_store, entry).await?;
		return resolve_in_repo_store(repo_store, prescans, entry_name).await;
	}

	// dir-rooted: an explicit `entry`, else the ancestor walk. On wasm the `fs`
	// store reads through the runner's fs globals, so a fs-less runtime cannot
	// resolve a dir-rooted entry at all.
	#[cfg(target_arch = "wasm32")]
	if !js_runtime::environment().has_fs() {
		bevybail!(
			"this runtime has no filesystem: pass a self-rooted `--repo` \
			(http:<prefix>, s3://<bucket>, r2://<binding>, indexed-db://<db>)"
		);
	}
	match entry {
		Some(entry) => {
			resolve_entry_path(prescans, repo_uri, store_fork, entry).await
		}
		None => discover_entry(prescans, repo_uri, store_fork).await,
	}
}

/// The store fork a launch naming none gets: a browser reading a remote repo
/// forks into IndexedDB (`indexed-db://beet/<repo prefix>`, one database per
/// origin with the repo's own prefix telling its forks apart), so a visitor's
/// first edit lands locally and every later boot reads it. Every other launch
/// reads its repo directly: a native process names its fork dir with
/// `--store-fork=fs:<dir>` when it wants one.
pub fn default_store_fork(repo_uri: &StoreUri) -> Option<StoreUri> {
	#[cfg(target_arch = "wasm32")]
	if repo_uri.is_remote()
		&& js_runtime::environment() == js_runtime::JsEnvironment::Browser
	{
		return Some(StoreUri::IndexedDb {
			name: "beet".into(),
			path_prefix: repo_uri.path_prefix().map(RelPath::new),
		});
	}
	let _ = repo_uri;
	None
}

/// The repo store `uri` names forked into `store_fork` (else
/// [`default_store_fork`]), see [`BlobStoreProvider::compose`].
fn compose_repo_store(
	uri: &StoreUri,
	store_fork: Option<&StoreUri>,
) -> Result<BlobStore> {
	let default = store_fork
		.is_none()
		.then(|| default_store_fork(uri))
		.flatten();
	BlobStoreProvider::compose(uri, store_fork.or(default.as_ref()))
}

/// The entry document a self-rooted store serves: `entry` when the launch names
/// one, else the first [`ENTRY_NAMES`] match at the store's root, erroring with
/// guidance on none. Shared by [`resolve_entry`] and a driver that heads the
/// document between builds (the Worker's version check).
pub async fn self_rooted_entry_name(
	repo_store: &BlobStore,
	entry: Option<&str>,
) -> Result<String> {
	match entry {
		Some(entry) => entry.to_string(),
		None => probe_entry_names(repo_store).await?.ok_or_else(|| {
			bevyhow!(
				"no entry document found in the `--repo` backend: looked \
				for {ENTRY_NAMES:?}. Seed one, or pass `--entry=<name>`."
			)
		})?,
	}
	.xok()
}

/// Walk the cwd and its ancestors for the first [`ENTRY_NAMES`] match, resolving
/// through an `fs` [`BlobStore`] at each candidate dir (consistent with the store
/// API and async, rather than a raw `fs_ext` probe). Discovery is the only place
/// a filesystem walk makes sense; the matched entry may still rebase its own
/// root ([`resolve_in_repo_store`]), and no match errors with guidance.
async fn discover_entry(
	prescans: &PrescanRegistry,
	repo_uri: Option<&StoreUri>,
	store_fork: Option<&StoreUri>,
) -> Result<ResolvedEntry> {
	let start = AbsPath::new(".")?;
	let mut dir = Some(start.clone());
	while let Some(current) = dir {
		let repo_store = BlobStore::new(FsStore::new(current.clone()));
		if let Some(entry_name) = probe_entry_names(&repo_store).await? {
			let repo_store = resolve_repo_store(repo_uri, store_fork, current)?;
			return resolve_in_repo_store(repo_store, prescans, entry_name)
				.await;
		}
		dir = current.parent();
	}
	bevybail!(
		"no entry document found: looked for {ENTRY_NAMES:?} in `{start}` and \
		its ancestors. Create a `main.bsx` or pass `--entry=<path>`."
	)
}

/// Resolve an explicit entry path (the binary's `--entry`, a command's `<entry>`
/// positional): a path with an extension names the entry file itself, anything
/// else is a directory probed for the first [`ENTRY_NAMES`] match. Either way
/// the entry may rebase its own store root with a `<RepoRoot src>` declaration
/// (see [`resolve_in_repo_store`]), the `--repo` param picks the backend and
/// `--store-fork` forks it into a local store.
pub async fn resolve_entry_path(
	prescans: &PrescanRegistry,
	repo_uri: Option<&StoreUri>,
	store_fork: Option<&StoreUri>,
	entry: &str,
) -> Result<ResolvedEntry> {
	let path = AbsPath::new(entry)?;
	let (repo_store, entry_name) = if path.extension().is_some() {
		// an entry file: its parent is the initial root
		let dir = path.parent().ok_or_else(|| {
			bevyhow!("entry `{path}` has no parent directory")
		})?;
		let entry_name = path
			.file_name()
			.ok_or_else(|| bevyhow!("entry `{path}` has no file name"))?
			.to_string();
		(resolve_repo_store(repo_uri, store_fork, dir)?, entry_name)
	} else {
		// a directory: probe it for an entry document
		let repo_store =
			resolve_repo_store(repo_uri, store_fork, path.clone())?;
		let entry_name =
			probe_entry_names(&repo_store).await?.ok_or_else(|| {
				bevyhow!(
					"no entry document found in `{path}`: looked for {ENTRY_NAMES:?}. \
				Create one, or name the entry file itself."
				)
			})?;
		(repo_store, entry_name)
	};
	resolve_in_repo_store(repo_store, prescans, entry_name).await
}

/// The first [`ENTRY_NAMES`] match at the store's root, if any.
pub async fn probe_entry_names(
	repo_store: &BlobStore,
) -> Result<Option<String>> {
	for name in ENTRY_NAMES {
		if repo_store.exists(&RelPath::from(*name)).await? {
			return Ok(Some(name.to_string()));
		}
	}
	Ok(None)
}

/// Build the [`BlobStore`] a `--repo` [`StoreUri`] names, its filesystem root
/// pinned to `dir` (the resolved entry directory, see [`StoreUri::rooted_at`])
/// and defaulting to a filesystem store there, forked into the `--store-fork`
/// store (likewise pinned). Shared by the binary's entry resolution
/// (the launch config's `--repo`) and the `check`/`serve`/`export-static`
/// commands (each command's own `--repo` param) so every entry load is
/// store-driven rather than filesystem-bound.
pub fn resolve_repo_store(
	repo_uri: Option<&StoreUri>,
	store_fork: Option<&StoreUri>,
	dir: AbsPath,
) -> Result<BlobStore> {
	let repo = repo_uri.cloned().unwrap_or_default().rooted_at(&dir);
	let store_fork = store_fork.map(|fork| fork.rooted_at(&dir));
	compose_repo_store(&repo, store_fork.as_ref())
}

/// The entry read from a store: its document bytes and name, and its
/// [`EntryPrescan`]. The world-free async read [`build_root`] consumes.
pub struct EntrySources {
	entry_name: String,
	entry: MediaBytes,
	prescan: EntryPrescan,
}

/// Read the entry document through `store`, awaited off the runtime (never
/// blocked, so it runs on the single-threaded Worker too). The caller hands over
/// the `prescan` entry resolution already produced, so the document is never
/// re-scanned.
pub async fn read_sources(
	repo_store: &BlobStore,
	entry_name: impl Into<String>,
	prescan: EntryPrescan,
) -> Result<EntrySources> {
	let entry_name = entry_name.into();
	let entry = repo_store
		.get_media(&RelPath::from(entry_name.as_str()))
		.await?;
	EntrySources {
		entry_name,
		entry,
		prescan,
	}
	.xok()
}

/// Build read [`EntrySources`] into a root carrying `repo_store` and its
/// [`PackageStore`] (resolved by ancestry for `<RoutesDir>` and the rest), with
/// `extra` riding onto the root. It carries no table store: an entry keeping
/// rows, ie a thread's records, declares one. The entry runs itself: its own `CallOnReady` verbs act on their `Ready`, so
/// a render-only command (`check`, `export-static`, the Worker) passes
/// [`DisableCallOnReady`] in `extra` to build the tree disarmed. The synchronous
/// world-mutating tail of an entry load; returns the root entity. A driver
/// passes [`RepoStore`] in `extra` to claim the built store as the process's
/// canonical one.
///
/// A build that fails leaves the root behind, carrying `extra` and the store
/// but no tree: the `--watch` driver's root stays a live-reload site the fix
/// latches onto, and the next build's teardown clears it like any other.
pub fn build_root(
	world: &mut World,
	repo_store: BlobStore,
	sources: EntrySources,
	extra: impl Bundle,
) -> Result<Entity> {
	let EntrySources {
		entry_name,
		entry,
		prescan,
	} = sources;
	let entry_path = RelPath::from(entry_name.as_str());
	let package = PackageStore::of_entry(&repo_store, &entry_path);
	// the root carries the store from the start (descendants resolve it by
	// ancestry) and `extra`, which is where the `RepoStore` marker rides rather
	// than landing here, since only a *driver* build (the process's own entry)
	// claims the app's one canonical store: a command loading a foreign entry
	// into the same world builds a second rooted store, which is that sub-app's,
	// not this process's.
	let root = world.spawn((extra, repo_store, package)).id();
	// the pre-scanned `<RequireCfg>`s, asserted before the tree builds: an unmet
	// one refuses the load, so nothing in the tree (a `CallOnReady` action) runs
	// in a binary that cannot support it, and the report holds even when the
	// tree itself cannot build (eg its root tag is not registered here).
	RequireCfg::assert_all(world, prescan.iter::<RequireCfg>())?;
	// its own path in its package, whose root is its directory
	let path = RelPath::new(entry_path.file_name().unwrap_or_default());
	let template = DocParsers::of(world)
		.read(&entry, Some(&path))
		.map_err(BevyError::from)
		.and_then(|source| DynamicTemplate::read(world, Arc::new(source)))
		.map_err(|err| {
			bevyhow!("failed to parse entry `{entry_name}`: {err}")
		})?;
	world
		.entity_mut(root)
		.insert_template(template)
		.map_err(|err| {
			bevyhow!("failed to load entry `{entry_name}`: {err}")
		})?;
	world.flush();
	Ok(root)
}

/// Build a resolved entry into an owned world and settle it to readiness: read
/// the sources through its store, build the root, then drive the async runtime
/// until every pending set drains ([`TemplatePending::settle_owned`]), so
/// `<RoutesDir>` scans and instantiated documents land before the caller serves. The
/// world-owning driver path (the wasm Worker, a one-shot build); an in-app caller
/// settles via [`TemplatePending::settle`] instead. Returns the entry root.
///
/// The build is disarmed ([`DisableCallOnReady`]): this driver serves each
/// request itself, so the entry's declared servers must not start.
#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
pub async fn build_entry_owned(
	world: &mut World,
	resolved: ResolvedEntry,
) -> Result<Entity> {
	let ResolvedEntry {
		repo_store,
		entry_name,
		prescan,
	} = resolved;
	let sources = read_sources(&repo_store, entry_name, prescan).await?;
	let root = build_root(
		world,
		repo_store,
		sources,
		(DisableCallOnReady, RepoStore),
	)?;
	TemplatePending::settle_owned(world).await;
	Ok(root)
}

/// The `--watch` entry build (native-only): install the live-reload driver
/// ([`EntryReloader`]), then do the first build through the same
/// [`rebuild_watched`] path a structural change re-runs (which also recomputes
/// the structural source set per build).
///
/// So editing the entry document or a document it instantiates tears the old
/// scene down and rebuilds it with no leaked entities (servers rebind, sockets
/// reconnect), while a markdown or layout edit keeps the light content re-fire. The whole of what
/// `beet --watch` runs after resolving its entry, so a test boots the exact
/// driver path.
#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
pub async fn build_watched(
	world: &AsyncWorld,
	repo_store: BlobStore,
	entry_name: String,
) -> Result {
	// the driver's rebuild callback, re-cloning the store and name per build
	// (it is an `Fn`, re-run on every structural change). The structural source
	// set starts empty; the first build below populates it.
	let rebuild = {
		let repo_store = repo_store.clone();
		let entry_name = entry_name.clone();
		move |world: AsyncWorld| -> LocalBoxedFuture<'static, Result> {
			let (repo_store, entry_name) =
				(repo_store.clone(), entry_name.clone());
			Box::pin(async move {
				rebuild_watched(&world, repo_store, entry_name).await
			})
		}
	};
	world
		.with(move |world: &mut World| {
			world.insert_resource(EntryReloader::new(default(), rebuild));
		})
		.await;
	// the first build: a no-op teardown, then the fresh `BeetSceneRoot`.
	rebuild_watched(world, repo_store, entry_name).await
}

/// Rebuild the `--watch` entry into a fresh [`BeetSceneRoot`], the shared path the
/// initial build and every structural reload run: tear down the previous entry
/// scene via [`BeetSceneRoot::despawn_all`] (servers close, sockets drop; a no-op on the first
/// build), re-read the sources through the store, and build a fresh root marked
/// [`BeetSceneRoot`] + [`LiveReload`] with its own entry [`WatchDir`]. The fresh
/// root's server children re-boot (rebinding their ports), so a browser's dropped
/// `/__client_io` socket reconnects and reloads into the new tree.
///
/// Once the tree settles, the structural source set is read off it
/// (`entry_source_paths`) and handed to the [`EntryReloader`], so a document this
/// very edit instantiates is structural on the next one without a restart. A
/// build that fails leaves the previous set in place, the entry in it, so the fix
/// rebuilds.
///
/// The [`EntryReloader`] resource (installed once) survives the teardown and drives
/// this on a change to the entry document or one it instantiates.
///
/// Gated on `client_io`: live reload is the browser channel plus the fs watcher,
/// so a binary that links neither simply has no `--watch`.
#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
pub async fn rebuild_watched(
	world: &AsyncWorld,
	repo_store: BlobStore,
	entry_name: String,
) -> Result {
	let prescans = world
		.with(|world: &mut World| {
			world.get_resource_or_init::<PrescanRegistry>().clone()
		})
		.await;
	let prescan = read_prescan(&repo_store, &prescans, &entry_name).await?;
	let sources =
		read_sources(&repo_store, entry_name.clone(), prescan).await?;
	// the entry is known before the build, so a first build that fails is
	// rebuilt by its fix
	let entry_path = RelPath::from(entry_name.as_str());
	let known = entry_path.clone();
	world
		.with(move |world: &mut World| {
			if let Some(mut reloader) =
				world.get_resource_mut::<EntryReloader>()
			{
				reloader.add_source(known);
			}
		})
		.await;
	let root = world
		.with(move |world: &mut World| -> Result<Entity> {
			// the entry's own dir, watched for edits to the entry doc / its
			// includes; on the root from the start, so a failed build's fix is
			// still seen.
			let entry_watch = WatchDir::for_entry(&repo_store, &entry_name);
			// tear down the previous entry scene so servers close and sockets drop
			// before the fresh tree binds (a no-op on the first build).
			BeetSceneRoot::despawn_all(world);
			// a rebuild retains nothing from the last one: the fresh tree runs
			// itself on its own `Ready`, so its servers rebind.
			let root = build_root(
				world,
				repo_store,
				sources,
				(
					BeetSceneRoot,
					LiveReload::new(),
					RepoStore,
					OnSpawn::insert_option(entry_watch),
				),
			)?;
			world.flush();
			Ok(root)
		})
		.await?;
	// a document instantiates asynchronously, so the tree names every one it
	// holds only once it has settled.
	TemplatePending::settle(world).await;
	world
		.with(move |world: &mut World| {
			let structural = entry_source_paths(world, root, &entry_path);
			if let Some(mut reloader) =
				world.get_resource_mut::<EntryReloader>()
			{
				reloader.set_sources(structural);
			}
		})
		.await;
	Ok(())
}

/// The structural entry sources whose change triggers a full rebuild (versus the
/// light content re-fire a markdown or layout edit gets), each store-root-relative
/// (matching the [`BlobEvent`] paths the watcher emits): the entry document and
/// every document the built tree under `root` instantiated, read off its
/// [`Activation`]s. One inside an excluded `bx:cfg` branch was never
/// instantiated, so is never among them.
///
/// The walk stops at a [`RoutesDir`]: a route's document builds per request
/// into a detached tree, so what it instantiates is a content source however
/// the routes are authored, and an edit to it must never promote to a full
/// rebuild.
#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
fn entry_source_paths(
	world: &mut World,
	root: Entity,
	entry: &RelPath,
) -> HashSet<RelPath> {
	let package = entry.parent().unwrap_or_default();
	let instantiated = world.with_state::<(
		Query<&Activation>,
		Query<&Children>,
		Query<(), With<RoutesDir>>,
	), _>(|(activations, children, routes_dirs)| {
		let mut stack = vec![root];
		let mut paths = Vec::new();
		while let Some(entity) = stack.pop() {
			paths.extend(
				activations
					.get(entity)
					.map(|activation| package.join(&activation.path)),
			);
			if routes_dirs.contains(entity) {
				continue;
			}
			stack.extend(children.get(entity).into_iter().flatten());
		}
		paths
	});
	instantiated.into_iter().chain([entry.clone()]).collect()
}

#[cfg(test)]
mod test {
	use super::*;

	/// The prescan set `world`'s plugins registered.
	fn prescans(world: &World) -> PrescanRegistry {
		world.resource::<PrescanRegistry>().clone()
	}

	/// The set the router registers, for a resolution with no world of its own.
	fn router_prescans() -> PrescanRegistry {
		prescans(&RouterPlugin.into_world())
	}

	/// Build `source` as the `main.bsx` entry of an in-memory store, through the
	/// same prescan, read and build steps a launch runs.
	async fn build_source(world: &mut World, source: &str) -> Result<Entity> {
		let repo_store = BlobStore::new(InMemoryBlobStore::new());
		repo_store
			.insert(&RelPath::from("main.bsx"), source.to_owned())
			.await?;
		let prescan =
			read_prescan(&repo_store, &prescans(world), "main.bsx").await?;
		let sources = read_sources(&repo_store, "main.bsx", prescan).await?;
		build_root(world, repo_store, sources, ())
	}

	/// The shared core builds an entry from any store: an in-memory store here, so
	/// it runs storage-agnostic (on wasm too), no filesystem involved. The entry's
	/// `<DefaultAppRoutes/>` lands on the built router root.
	#[beet_core::test]
	async fn builds_an_entry_from_an_in_memory_store() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root =
			build_source(&mut world, "<Router><DefaultAppRoutes/></Router>")
				.await
				.unwrap();
		// its store is bytes alone: an entry keeping rows declares a table store
		world.entity(root).contains::<BlobStore>().xpect_true();
		world.entity(root).contains::<TableStore>().xpect_false();
		// the entry built into a router root carrying the default app routes
		world.entity(root).contains::<Router>().xpect_true();
		world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["app-info"])
			.xpect_some();
	}

	/// The readiness gate settles and returns once the entry has nothing pending.
	/// This entry has no `<RoutesDir>` and instantiates nothing, so it is ready the
	/// moment `build_root` returns; the gate must return rather than hang.
	#[beet_core::test]
	async fn gate_settles_when_ready() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		build_source(&mut world, "<Router/>").await.unwrap();
		// returns rather than hanging, nothing being pending on this entry.
		TemplatePending::settle_owned(&mut world).await;
	}

	/// An unmet `<RequireCfg>` refuses the load before the tree builds, nested
	/// in the root element or beside it: nothing it declares spawns, so no
	/// `CallOnReady` action runs in a binary that cannot support it (the
	/// windowed `wgpu-action.bsx` drove its fox on a headless binary).
	#[beet_core::test]
	async fn unmet_requirement_refuses_the_build() {
		for source in [
			r#"<Router><RequireCfg cfg="feature:nosuch"/><DefaultAppRoutes/></Router>"#,
			r#"<RequireCfg cfg="feature:nosuch"/><Router><DefaultAppRoutes/></Router>"#,
		] {
			let mut world = (AsyncPlugin, RouterPlugin).into_world();
			build_source(&mut world, source)
				.await
				.unwrap_err()
				.to_string()
				.xpect_contains("feature:nosuch");
			world
				.query_filtered::<(), Or<(With<Router>, With<RequireCfg>)>>()
				.iter(&world)
				.count()
				.xpect_eq(0);
		}
	}

	/// An entry is a document like any other, so it may have several roots, a
	/// declaration beside the rest among them, each built under a bare root.
	#[beet_core::test]
	async fn several_roots_build() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.spawn(
			CrateRegistration::new("beet-cli", "0.0.9")
				.with_feature("extra")
				.with_skip_prefix(),
		);
		let root = build_source(
			&mut world,
			r#"<RequireCfg cfg="feature:extra"/>
			<Router><DefaultAppRoutes/></Router>
			<Sequence/>
			<RequireCfg cfg="version:0.0.1" bx:cfg="version:0.0.1"/>"#,
		)
		.await
		.unwrap();
		world.entity(root).contains::<Router>().xpect_false();
		world.get::<Children>(root).unwrap().len().xpect_eq(4);
		// the site is found under the root however many it has
		RouteTree::of(&world, root)
			.unwrap()
			.find(&["app-info"])
			.xpect_some();
	}

	/// Every root declaring `CallOnReady` runs: nothing calls an entry's root.
	#[beet_core::test]
	async fn each_armed_root_runs() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		build_source(
			&mut world,
			"<Sequence {CallOnReady}/><Sequence/><Sequence {CallOnReady}/>",
		)
		.await
		.unwrap();
		let calls = world
			.query_filtered::<(), With<CallOnReady>>()
			.iter(&world)
			.count();
		calls.xpect_eq(2);
		AsyncRunner::settle_async_tasks(&mut world).await;
		world.resource::<Messages<AppExit>>().len().xpect_eq(calls);
	}

	/// The entry's declared document lands in the process environment as the
	/// entry resolves, before anything builds: an identity handed in through
	/// the environment opens it, and a record already set is left alone.
	#[cfg(all(feature = "vault", not(target_arch = "wasm32")))]
	#[beet_core::test]
	async fn resolution_loads_the_entry_document() {
		let identity = AgeIdentity::generate();
		let mut identities = AgeIdentityFile::default();
		identities.push(identity.clone());
		let mut document = SecretsDocument::default();
		for name in ["BEET_TEST_LAUNCH_LOADED", "BEET_TEST_LAUNCH_KEPT"] {
			document
				.set(
					&identities,
					"default",
					name,
					"from-document",
					SecretRecord {
						role: Some(SecretRole::EnvVar),
						..default()
					},
				)
				.unwrap();
		}
		let repo_store = BlobStore::new(InMemoryBlobStore::new());
		repo_store
			.insert(
				&RelPath::from("app/main.bsx"),
				"<Router><RepoRoot src=\"..\"/><Secrets/></Router>",
			)
			.await
			.unwrap();
		SecretsHandle::new(repo_store.clone(), "secrets.toml")
			.unwrap()
			.write(&document)
			.await
			.unwrap();
		// SAFETY: test-only, names no other test reads; the identity is the
		// process's for the load
		let previous = env_ext::var(AgeIdentityFile::ENV_VAR).ok();
		unsafe {
			env_ext::set_var(AgeIdentityFile::ENV_VAR, &identity.to_string())
				.unwrap();
			env_ext::set_var("BEET_TEST_LAUNCH_KEPT", "from-shell").unwrap();
		}
		resolve_in_repo_store(
			repo_store,
			&router_prescans(),
			"app/main.bsx".into(),
		)
		.await
		.unwrap();
		env_ext::var("BEET_TEST_LAUNCH_LOADED")
			.unwrap()
			.xpect_eq("from-document");
		env_ext::var("BEET_TEST_LAUNCH_KEPT")
			.unwrap()
			.xpect_eq("from-shell");
		unsafe {
			match previous {
				Some(value) => {
					env_ext::set_var(AgeIdentityFile::ENV_VAR, &value)
				}
				None => env_ext::remove_var(AgeIdentityFile::ENV_VAR),
			}
			.unwrap();
			env_ext::remove_var("BEET_TEST_LAUNCH_LOADED").unwrap();
			env_ext::remove_var("BEET_TEST_LAUNCH_KEPT").unwrap();
		}
	}

	/// An fs entry declaring `<RepoRoot src="..">` re-roots the store at the
	/// resolved ancestor directory: the watch dir is the widened root and the
	/// entry name grows the path back down to the document.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn fs_entry_rebases_through_resolve_entry_path() {
		let tmp = TempDir::new().unwrap();
		let entry_dir = tmp.path().join("app");
		fs_ext::create_dir_all(&entry_dir).unwrap();
		fs_ext::write(
			entry_dir.join("main.bsx"),
			"<Router><RepoRoot src=\"..\"/></Router>",
		)
		.unwrap();
		let resolved = resolve_entry_path(
			&router_prescans(),
			None,
			None,
			entry_dir.as_str(),
		)
		.await
		.unwrap();
		resolved.entry_name.xpect_eq("app/main.bsx");
		resolved.watch_dir.xpect_eq(Some(tmp.path().clone()));
		resolved
			.repo_store
			.exists(&RelPath::from("app/main.bsx"))
			.await
			.unwrap()
			.xpect_true();
	}

	/// A store with no parent universe honours the same declaration as a
	/// key-prefix view of itself; the binary's self-rooted branch resolves
	/// through this same [`resolve_in_repo_store`], so the declaration is never
	/// dropped by policy.
	#[beet_core::test]
	async fn self_rooted_entry_rebases_to_a_prefix_view() {
		let repo_store = BlobStore::new(InMemoryBlobStore::new());
		repo_store
			.insert(
				&RelPath::from("apps/site/main.bsx"),
				"<Router><RepoRoot src=\"..\"/></Router>",
			)
			.await
			.unwrap();
		let resolved = resolve_in_repo_store(
			repo_store,
			&router_prescans(),
			"apps/site/main.bsx".to_string(),
		)
		.await
		.unwrap();
		resolved.entry_name.xpect_eq("site/main.bsx");
		#[cfg(not(target_arch = "wasm32"))]
		resolved.watch_dir.xpect_none();
		resolved
			.repo_store
			.exists(&RelPath::from("site/main.bsx"))
			.await
			.unwrap()
			.xpect_true();
	}

	/// A root-level entry declaring a root above a store with no parent
	/// universe fails loudly naming the mis-publish.
	#[beet_core::test]
	async fn self_rooted_escape_fails_loudly() {
		let repo_store = BlobStore::new(InMemoryBlobStore::new());
		repo_store
			.insert(
				&RelPath::from("main.bsx"),
				"<Router><RepoRoot src=\"../..\"/></Router>",
			)
			.await
			.unwrap();
		resolve_in_repo_store(
			repo_store,
			&router_prescans(),
			"main.bsx".to_string(),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("mis-published");
	}

	/// The binary path and the command path share [`resolve_in_repo_store`], so the
	/// same inputs resolve an identical `(store, entry_name)`: here the
	/// command-shaped `resolve_entry_path` against the binary-shaped store + name
	/// pair.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn both_paths_resolve_identically() {
		let tmp = TempDir::new().unwrap();
		fs_ext::write(
			tmp.path().join("main.bsx"),
			"<Router><RepoRoot src=\".\"/></Router>",
		)
		.unwrap();
		let prescans = router_prescans();
		let by_path =
			resolve_entry_path(&prescans, None, None, tmp.path().as_str())
				.await
				.unwrap();
		let by_store = resolve_in_repo_store(
			resolve_repo_store(None, None, tmp.path().clone()).unwrap(),
			&prescans,
			"main.bsx".to_string(),
		)
		.await
		.unwrap();
		by_path.entry_name.xpect_eq(by_store.entry_name);
		by_path
			.repo_store
			.same_scope(&by_store.repo_store)
			.xpect_true();
		by_path.watch_dir.xpect_eq(by_store.watch_dir);
	}

	/// A store fork composes over the resolved repo: a launch on a remote repo
	/// with `--store-fork` composes the pair, reading the entry through the
	/// upstream and holding the fork locally, on a self-rooted repo and on a
	/// dir-rooted one alike.
	#[beet_core::test]
	async fn a_store_fork_composes_over_the_repo() {
		let upstream = StoreUri::parse("memory://fork-upstream").unwrap();
		let local = StoreUri::parse("memory://fork-local").unwrap();
		let seeded = BlobStore::from_uri(&upstream).unwrap();
		seeded
			.insert(&RelPath::from("main.bsx"), "<Router/>")
			.await
			.unwrap();
		let prescans = router_prescans();
		let resolved =
			resolve_entry(&prescans, Some(&upstream), Some(&local), None)
				.await
				.unwrap();
		resolved.entry_name.xpect_eq("main.bsx");
		resolved.repo_store.id().xpect_eq("fork");
		resolved
			.repo_store
			.insert(&RelPath::from("fork.json"), "{}")
			.await
			.unwrap();
		BlobStore::from_uri(&local)
			.unwrap()
			.exists(&RelPath::from("fork.json"))
			.await
			.unwrap()
			.xpect_true();
		seeded
			.exists(&RelPath::from("fork.json"))
			.await
			.unwrap()
			.xpect_false();
		// no store fork natively: the repo is read directly
		resolve_entry(&prescans, Some(&upstream), None, None)
			.await
			.unwrap()
			.repo_store
			.id()
			.xpect_eq("memory");
	}

	/// Every `--watch` rebuild fires a fresh [`Ready`] on its fresh entry root,
	/// so a rebuilt tree boots exactly as the first one did: nothing is retained
	/// between builds.
	#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
	#[beet_core::test]
	async fn rebuild_fires_ready_every_time() {
		let repo_store = BlobStore::new(InMemoryBlobStore::new());
		repo_store
			.insert(&RelPath::from("main.bsx"), "<Router/>")
			.await
			.unwrap();
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let readies = Store::new(0);
		let recorder = readies.clone();
		// the entry root is the one marked `BeetSceneRoot`.
		world.add_observer(
			move |ev: On<Ready>, entries: Query<(), With<BeetSceneRoot>>| {
				if entries.contains(ev.entity) {
					recorder.set(recorder.get() + 1);
				}
			},
		);
		world
			.run_async_local_then(move |world| async move {
				for _ in 0..2 {
					rebuild_watched(
						&world,
						repo_store.clone(),
						"main.bsx".to_string(),
					)
					.await
					.unwrap();
				}
			})
			.await;
		readies.get().xpect_eq(2);
	}

	/// A rebuild that fails (a typo in the entry) still leaves exactly one
	/// [`LiveReload`] site carrying the store, so the fix latches a reload on
	/// it and the next rebuild tears it down: the dev loop survives a broken
	/// save rather than needing a restart.
	#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
	#[beet_core::test]
	async fn failed_rebuild_keeps_the_site_reloadable() {
		let repo_store = BlobStore::new(InMemoryBlobStore::new());
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		// per rebuild: whether it built, and the live sites it left behind
		let outcomes = Store::new(Vec::new());
		let recorder = outcomes.clone();
		let store = repo_store.clone();
		world
			.run_async_local_then(move |world| async move {
				for source in ["<Router/>", "<Router {", "<Router/>"] {
					store
						.insert(&RelPath::from("main.bsx"), source)
						.await
						.unwrap();
					let built = rebuild_watched(
						&world,
						store.clone(),
						"main.bsx".to_string(),
					)
					.await
					.is_ok();
					let sites = world
						.with(|world| {
							world.with_state::<Query<
								(),
								(
									With<LiveReload>,
									With<BlobStore>,
									With<BeetSceneRoot>,
								),
							>, _>(|query| query.iter().count())
						})
						.await;
					recorder.push((built, sites));
				}
			})
			.await;
		outcomes
			.get()
			.xpect_eq(vec![(true, 1), (false, 1), (true, 1)]);
	}

	/// The structural set is read off the built tree: the entry and every
	/// document it instantiated, transitively. A `Layout.bsx` only a per-request
	/// route uses is never in the tree, so it stays a content source, as does the
	/// route it wraps.
	#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
	#[beet_core::test]
	async fn structural_sources_include_entry_documents() {
		let repo_store = BlobStore::new(InMemoryBlobStore::new());
		let files = [
			(
				"app/main.bsx",
				r#"<Router {Layout(Document("templates/Layout.bsx"))}><Instance document="templates/Styles.bsx"/><RoutesDir src="app/routes"/></Router>"#,
			),
			(
				"app/templates/Styles.bsx",
				r#"<Instance document="./widgets/Swatch"/>"#,
			),
			("app/templates/widgets/Swatch.bsx", "<i class=\"a\"/>"),
			("app/templates/Layout.bsx", "<html><slot/></html>"),
			("app/routes/index.md", "# Home"),
		];
		for (path, source) in files {
			repo_store
				.insert(&RelPath::from(path), source)
				.await
				.unwrap();
		}
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(EntryReloader::new(default(), |_| {
			Box::pin(async { Ok(()) })
		}));
		world
			.run_async_local_then(move |world| async move {
				rebuild_watched(&world, repo_store, "app/main.bsx".to_string())
					.await
					.unwrap();
			})
			.await;
		let reloader = world.resource::<EntryReloader>();
		files
			.iter()
			.map(|(path, _)| RelPath::from(*path))
			.filter(|path| reloader.is_structural(path))
			.collect::<HashSet<_>>()
			.xpect_eq(
				[
					"app/main.bsx",
					"app/templates/Styles.bsx",
					"app/templates/widgets/Swatch.bsx",
				]
				.into_iter()
				.map(RelPath::from)
				.collect::<HashSet<_>>(),
			);
	}

	/// An entry instantiating a document, in a directory of its own: the
	/// document resolves from there and an instance is the action its route
	/// dispatches through.
	#[beet_core::test]
	async fn an_entry_instantiates_documents() {
		let repo_store = BlobStore::new(InMemoryBlobStore::new());
		for (path, source) in [
			(
				"main.bsx",
				r#"<Router><Instance document="./routes"/></Router>"#,
			),
			("routes.bsx", r#"<Route path="hello"/>"#),
		] {
			repo_store
				.insert(&RelPath::from(path), source)
				.await
				.unwrap();
		}
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let prescan = read_prescan(&repo_store, &prescans(&world), "main.bsx")
			.await
			.unwrap();
		let sources = read_sources(&repo_store, "main.bsx", prescan)
			.await
			.unwrap();
		let root = build_root(&mut world, repo_store, sources, ()).unwrap();
		TemplatePending::settle_owned(&mut world).await;
		world.get::<TemplateError>(root).xpect_none();
		world
			.with_state::<Query<&Activation>, _>(|query| query.iter().count())
			.xpect_eq(1);
	}
}
