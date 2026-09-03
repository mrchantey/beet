//! The cross-platform entry resolution + build core shared by the native `beet`
//! binary, the wasm Worker entry, and the `check`/`serve`/`export-static`
//! commands.
//!
//! An entry load splits into resolution ([`resolve_main`]: the store + the entry
//! document name within it, honouring `--repo` and the entry's own
//! `<RepoRoot src>`), a world-free async read ([`read_sources`]: the
//! entry document and the templates under its declared `<TemplateDir>`s, through the
//! [`BlobStore`]) and a synchronous world build ([`build_root`]: register the
//! templates, parse the entry, build it into a root carrying the store). The entry's
//! own template dirs are registered *before* the entry parses, so entry-level tags
//! (eg `<Styles/>`) resolve; the reactive `<TemplateDir>` observer covers everything
//! that loads later (route pages, library widgets). The same path runs on the native
//! async runtime and the single-threaded wasm Worker, so an entry build never
//! requires a filesystem.

use beet::prelude::*;

/// Entry-document file names discovery looks for, in priority order. The native
/// binary walks the cwd and its ancestors for the first match; the `check`/`serve`/
/// `export-static` commands search a single given site dir for it. Shared so both
/// agree on what an entry document is named.
pub const ENTRY_NAMES: &[&str] = &["main.bsx", "main.json", "main.ron"];

/// The binary's own [`CrateRegistration`]: every feature `beet-cli` can be
/// compiled with, each recorded if enabled, so an entry's `<CrateCheck/>` and
/// the `--features` flag verify against the running binary. Spawned by every
/// entry driver (the native binary, the wasm binary, the Worker).
pub fn cli_registration() -> CrateRegistration {
	crate_registration!({
		features: [
			"aws_sdk",
			"cloudflare",
			"extra",
			"geoip",
			"infra",
			"lambda",
			"ml",
			"net",
			"pdf",
			"qrcode",
			"secure",
			"sockets",
			"ssh",
			"thread",
			"tui",
			"web",
			"web_examples",
			"web_head",
			"winit",
		]
	})
	.with_skip_prefix()
}

/// Pre-scan the raw entry document through `store`, the one registry-free walk
/// entry resolution reads its declarations from.
pub async fn read_prescan(
	repo_store: &BlobStore,
	entry_name: &str,
) -> Result<EntryPrescan> {
	repo_store
		.get_media(&SmolPath::from(entry_name))
		.await?
		.xmap(|entry| EntryPrescan::parse(&entry))
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
	/// store, so the load never re-parses it.
	pub prescan: EntryPrescan,
	#[cfg(not(target_arch = "wasm32"))]
	pub watch_dir: Option<AbsPathBuf>,
}

/// Resolve an entry within its store: read the prescan once, and rebase the
/// store through the entry's own `<RepoRoot src>` declaration
/// ([`BlobStore::rebase_repo`]) when it carries one. The one widening path
/// every entry load shares (the binary, discovery, `serve`/`check`/
/// `export-static`, the Worker); callers differ only in how the initial
/// `(store, entry_name)` pair is derived (a local path walk vs a key in a
/// self-rooted store).
///
/// Live reload watches the store's local root when it has one
/// ([`BlobStoreProvider::watch_dir`]) — the rebased root, so the watcher sees
/// the entry's whole declared universe; a store with no local directory
/// watches nothing.
pub async fn resolve_in_repo_store(
	repo_store: BlobStore,
	entry_name: String,
) -> Result<ResolvedEntry> {
	let prescan = read_prescan(&repo_store, &entry_name).await?;
	// the rebased store holds the same entry document, so its pre-scan is the
	// one already read: entry resolution parses the entry exactly once.
	let (repo_store, entry_name) = match &prescan.repo_root {
		Some(src) => repo_store.rebase_repo(&entry_name, src)?,
		None => (repo_store, entry_name),
	};
	Ok(ResolvedEntry {
		#[cfg(not(target_arch = "wasm32"))]
		watch_dir: repo_store.watch_dir(),
		repo_store,
		entry_name,
		prescan,
	})
}

/// Resolve an explicit entry path (the binary's `--main`, a command's `<entry>`
/// positional): a path with an extension names the entry file itself, anything
/// else is a directory probed for the first [`ENTRY_NAMES`] match. Either way
/// the entry may rebase its own store root with a `<RepoRoot src>` declaration
/// (see [`resolve_in_repo_store`]), and the `--repo` param picks the backend.
pub async fn resolve_main(
	repo_uri: Option<&StoreUri>,
	main: &str,
) -> Result<ResolvedEntry> {
	let path = AbsPathBuf::new(main)?;
	let (repo_store, entry_name) = if path.extension().is_some() {
		// an entry file: its parent is the initial root
		let dir = path.parent().ok_or_else(|| {
			bevyhow!("entry `{path}` has no parent directory")
		})?;
		let entry_name = path
			.file_name()
			.and_then(|name| name.to_str())
			.ok_or_else(|| bevyhow!("entry `{path}` has no file name"))?
			.to_string();
		(resolve_repo_store(repo_uri, dir)?, entry_name)
	} else {
		// a directory: probe it for an entry document
		let repo_store = resolve_repo_store(repo_uri, path.clone())?;
		let entry_name =
			probe_entry_names(&repo_store).await?.ok_or_else(|| {
				bevyhow!(
					"no entry document found in `{path}`: looked for {ENTRY_NAMES:?}. \
				Create one, or name the entry file itself."
				)
			})?;
		(repo_store, entry_name)
	};
	resolve_in_repo_store(repo_store, entry_name).await
}

/// The first [`ENTRY_NAMES`] match at the store's root, if any.
pub async fn probe_entry_names(
	repo_store: &BlobStore,
) -> Result<Option<String>> {
	for name in ENTRY_NAMES {
		if repo_store.exists(&SmolPath::from(*name)).await? {
			return Ok(Some(name.to_string()));
		}
	}
	Ok(None)
}

/// Build the [`BlobStore`] a `--repo` [`StoreUri`] names, defaulting to a
/// filesystem store rooted at `dir` (the resolved entry directory). Shared by
/// the binary's entry resolution (the launch config's `--repo`) and the
/// `check`/`serve`/`export-static` commands (each command's own `--repo`
/// param) so every entry load is store-driven rather than filesystem-bound.
pub fn resolve_repo_store(
	repo_uri: Option<&StoreUri>,
	dir: AbsPathBuf,
) -> Result<BlobStore> {
	BlobStore::from_uri(repo_uri.unwrap_or(&StoreUri::default()), dir)
}

/// The entry sources read from a store: the entry document bytes + name, its
/// [`EntryPrescan`], the template `(path, source)` pairs from its declared
/// `<TemplateDir>`s, and the formats they register through. The world-free async
/// read [`build_root`] consumes.
pub struct EntrySources {
	entry_name: String,
	entry: MediaBytes,
	prescan: EntryPrescan,
	template_sources: Vec<(SmolPath, String)>,
	formats: TemplateFormats,
}

/// Read the entry document and the templates under its declared `<TemplateDir>`s
/// through `store`, awaited off the runtime (never blocked, so it runs on the
/// single-threaded Worker too). The caller reads `formats` from the world first,
/// since the read itself is world-free, and hands over the `prescan` entry
/// resolution already produced, so the document is never re-parsed.
pub async fn read_sources(
	repo_store: &BlobStore,
	formats: TemplateFormats,
	entry_name: impl Into<String>,
	prescan: EntryPrescan,
) -> Result<EntrySources> {
	let entry_name = entry_name.into();
	let entry = repo_store
		.get_media(&SmolPath::from(entry_name.as_str()))
		.await?;
	// a markup entry may declare `<TemplateDir>`s naming template directories; read
	// each so they register before the entry parses (so entry-level tags resolve). A
	// non-markup (serde) entry declares none.
	let mut template_sources = Vec::new();
	for dir in &prescan.template_dirs {
		template_sources.extend(
			TemplateDir::read_sources(repo_store, dir, &formats).await?,
		);
	}
	EntrySources {
		entry_name,
		entry,
		prescan,
		template_sources,
		formats,
	}
	.xok()
}

/// Build read [`EntrySources`] into a root carrying `store` (resolved by ancestry for
/// `<TemplateDir>`, `<RoutesDir>` and `<Template src>`), with `extra` riding onto the
/// root. The entry runs itself: its own `CallOnReady` verbs act on their `Ready`, so
/// a render-only command (`check`, `export-static`, the Worker) passes
/// [`DisableCallOnReady`] in `extra` to build the tree disarmed. Registers the
/// entry's declared template sources *before* parsing the entry (so its own tags
/// resolve), then marks the root [`TemplatesLoaded`]. The synchronous
/// world-mutating tail of an entry load; returns the root entity. A driver
/// passes [`RepoStore`] in `extra` to claim the built store as the process's
/// canonical one.
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
		template_sources,
		formats,
	} = sources;
	// the pre-scanned `<CrateCheck>`s, spawned before the tree builds so a check
	// fires with its missing-feature list even when the tree itself cannot build
	// (eg its root tag is feature-gated out of this binary).
	if !prescan.checks.is_empty() {
		for check in prescan.checks {
			world.spawn(check);
		}
		world.flush();
	}
	// the root is spawned first so it can own the entry-level template
	// registrations: tearing the entry scene down (a structural live reload)
	// unregisters them with it, so no stale template survives a rebuild.
	let root = world.spawn(()).id();
	// the entry's own template dirs, registered before the entry parses so its
	// entry-level tags (eg `<Styles/>`) resolve. The reactive `<TemplateDir>` observer
	// re-registers them (plus any crate/route dirs) once the tree is built.
	TemplateDir::register_sources(world, root, &formats, template_sources)?;
	let template = EntryTemplate::from_bytes(world, &entry).map_err(|err| {
		bevyhow!("failed to parse entry `{entry_name}`: {err}")
	})?;
	// the store on the root: descendants resolve it by ancestry. The `RepoStore`
	// marker rides `extra` rather than landing here, since only a *driver* build
	// (the process's own entry) claims the app's one canonical store: a command
	// loading a foreign entry into the same world builds a second rooted store,
	// which is that sub-app's, not this process's. `TemplatesLoaded` marks the
	// entry-level templates registered (the readiness signal a wasm Worker waits
	// on before serving).
	let mut root_entity = world.entity_mut(root);
	root_entity.insert((extra, repo_store, TemplatesLoaded));
	root_entity.insert_template(template).map_err(|err| {
		bevyhow!("failed to load entry `{entry_name}`: {err}")
	})?;
	world.flush();
	Ok(root)
}

/// Build an entry into an owned world and settle it to readiness: read the
/// sources through `store`, build the root, then drive the async runtime until
/// every pending set drains ([`TemplatePending::settle_owned`]), so
/// `<RoutesDir>`/`<TemplateDir>` scans land before the caller serves. The
/// world-owning driver path (the wasm Worker, a one-shot build); an in-app caller
/// settles via [`TemplatePending::settle`] instead. Returns the entry root.
///
/// The build is disarmed ([`DisableCallOnReady`]): this driver serves each
/// request itself, so the entry's declared servers must not start.
#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
pub(crate) async fn build_entry_owned(
	world: &mut World,
	repo_store: BlobStore,
	entry_name: String,
) -> Result<Entity> {
	let formats = world.get_resource_or_init::<TemplateFormats>().clone();
	// the shared resolution: the entry's `<RepoRoot>` rebases the bucket view
	// exactly as it does every other store kind.
	let ResolvedEntry {
		repo_store,
		entry_name,
		prescan,
	} = resolve_in_repo_store(repo_store, entry_name).await?;
	let sources =
		read_sources(&repo_store, formats, entry_name, prescan).await?;
	let root = build_root(
		world,
		repo_store,
		sources,
		(DisableCallOnReady, RepoStore),
	)?;
	TemplatePending::settle_owned(world).await;
	Ok(root)
}

/// Rebuild the `--watch` entry into a fresh [`BeetSceneRoot`], the shared path the
/// initial build and every structural reload run: tear down the previous entry
/// scene via [`BeetSceneRoot::despawn_all`] (servers close, sockets drop; a no-op on the first
/// build), re-read the sources through the store, and build a fresh root marked
/// [`BeetSceneRoot`] + [`LiveReload`] with its own entry [`WatchDir`]. The fresh
/// root's server children re-boot (rebinding their ports), so a browser's dropped
/// `/__client_io` socket reconnects and reloads into the new tree.
///
/// The [`EntryReloader`] resource (installed once) survives the teardown and drives
/// this on a change to the entry document or an included `<Template src>`.
#[cfg(not(target_arch = "wasm32"))]
pub async fn rebuild_watched(
	world: &AsyncWorld,
	repo_store: BlobStore,
	entry_name: String,
	formats: TemplateFormats,
) -> Result {
	let prescan = read_prescan(&repo_store, &entry_name).await?;
	let sources =
		read_sources(&repo_store, formats, entry_name.clone(), prescan).await?;
	// recompute the structural source set from the current content, so a
	// `<Template src>` include added by this very edit is structural on the next
	// one without a restart.
	let structural = entry_source_paths(&repo_store, &entry_name).await;
	world
		.with(move |world: &mut World| -> Result {
			if let Some(mut reloader) =
				world.get_resource_mut::<EntryReloader>()
			{
				reloader.set_sources(structural);
			}
			// the entry's own dir, watched for edits to the entry doc / its includes;
			// computed before `build_root` consumes `store`.
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
				(BeetSceneRoot, LiveReload::new(), RepoStore),
			)?;
			if let Some(entry_watch) = entry_watch {
				world.entity_mut(root).insert(entry_watch);
			}
			world.flush();
			Ok(())
		})
		.await
}

/// The structural entry sources whose change triggers a full rebuild (versus the
/// light content re-fire a markdown/template edit gets): the entry document plus
/// every `<Template src>` include, resolved transitively through the store. Every
/// path is store-root-relative, matching the [`BlobEvent`] paths the watcher emits.
///
/// A missing / unreadable / non-markup source is skipped rather than erroring, so a
/// broken include never blocks watch startup.
#[cfg(not(target_arch = "wasm32"))]
async fn entry_source_paths(
	repo_store: &BlobStore,
	entry_name: &str,
) -> HashSet<SmolPath> {
	let mut seen = HashSet::default();
	let mut stack = vec![SmolPath::from(entry_name)];
	while let Some(path) = stack.pop() {
		if !seen.insert(path.clone()) {
			continue;
		}
		let Ok(media) = repo_store.get_media(&path).await else {
			continue;
		};
		stack.extend(
			EntryPrescan::parse_lossy(&media)
				.includes
				.into_iter()
				.map(|src| SmolPath::from(src.as_str())),
		);
	}
	seen
}

/// Build an entry from in-memory BSX text rather than a store read: the browser
/// path, where the program is inlined in a `<script type="application/x-bsx">`, not
/// resolved from `--main`/a filesystem. Constructs [`EntrySources`] directly and
/// builds onto an in-memory ([`BlobStore::temp`]) repo store, so the same
/// [`build_root`] core runs as the store-backed native path.
pub fn build_from_bsx(
	world: &mut World,
	formats: TemplateFormats,
	entry_name: impl Into<String>,
	bsx: impl Into<String>,
) -> Result<Entity> {
	let entry = MediaBytes::new_bsx(bsx.into());
	let sources = EntrySources {
		entry_name: entry_name.into(),
		prescan: EntryPrescan::parse(&entry)?,
		entry,
		template_sources: Vec::new(),
		formats,
	};
	build_root(world, BlobStore::temp(), sources, RepoStore)
}

#[cfg(test)]
mod test {
	use super::*;

	/// The shared core builds an entry from any store: an in-memory store here, so
	/// it runs storage-agnostic (on wasm too), no filesystem involved. The entry's
	/// `<DefaultAppRoutes/>` lands on the built router root.
	#[beet::test]
	async fn builds_an_entry_from_an_in_memory_store() {
		let repo_store = BlobStore::temp();
		repo_store
			.insert(
				&SmolPath::from("main.bsx"),
				"<Router><DefaultAppRoutes/></Router>",
			)
			.await
			.unwrap();
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let formats = world.get_resource_or_init::<TemplateFormats>().clone();
		let prescan = read_prescan(&repo_store, "main.bsx").await.unwrap();
		let sources = read_sources(&repo_store, formats, "main.bsx", prescan)
			.await
			.unwrap();
		let root = build_root(&mut world, repo_store, sources, ()).unwrap();
		// the entry built into a router root carrying the default app routes
		world.entity(root).contains::<Router>().xpect_true();
		world
			.entity(root)
			.get::<RouteTree>()
			.unwrap()
			.find(&["js", "reactivity.js"])
			.xpect_some();
	}

	/// The readiness gate settles and returns once the entry has nothing pending.
	/// This entry has no `<RoutesDir>`/`<TemplateDir>`, so it is ready the moment
	/// `build_root` returns; the gate must return rather than hang.
	#[beet::test]
	async fn gate_settles_when_ready() {
		let repo_store = BlobStore::temp();
		repo_store
			.insert(&SmolPath::from("main.bsx"), "<Router/>")
			.await
			.unwrap();
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let formats = world.get_resource_or_init::<TemplateFormats>().clone();
		let prescan = read_prescan(&repo_store, "main.bsx").await.unwrap();
		let sources = read_sources(&repo_store, formats, "main.bsx", prescan)
			.await
			.unwrap();
		let root = build_root(&mut world, repo_store, sources, ()).unwrap();
		world
			.entity(root)
			.contains::<TemplatesLoaded>()
			.xpect_true();
		// returns rather than hanging, nothing being pending on this entry.
		TemplatePending::settle_owned(&mut world).await;
	}

	/// An fs entry declaring `<RepoRoot src="..">` re-roots the store at the
	/// resolved ancestor directory: the watch dir is the widened root and the
	/// entry name grows the path back down to the document.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet::test]
	async fn fs_entry_rebases_through_resolve_main() {
		let tmp = TempDir::new().unwrap();
		let entry_dir = tmp.path().join("app");
		fs_ext::create_dir_all(&entry_dir).unwrap();
		fs_ext::write(
			entry_dir.join("main.bsx"),
			"<Router><RepoRoot src=\"..\"/></Router>",
		)
		.unwrap();
		let resolved = resolve_main(None, entry_dir.to_string_lossy().as_ref())
			.await
			.unwrap();
		resolved.entry_name.xpect_eq("app/main.bsx");
		resolved.watch_dir.xpect_eq(Some(tmp.path().clone()));
		resolved
			.repo_store
			.exists(&SmolPath::from("app/main.bsx"))
			.await
			.unwrap()
			.xpect_true();
	}

	/// A store with no parent universe honours the same declaration as a
	/// key-prefix view of itself; the binary's self-rooted branch resolves
	/// through this same [`resolve_in_repo_store`], so the declaration is never
	/// dropped by policy.
	#[beet::test]
	async fn self_rooted_entry_rebases_to_a_prefix_view() {
		let repo_store = BlobStore::temp();
		repo_store
			.insert(
				&SmolPath::from("apps/site/main.bsx"),
				"<Router><RepoRoot src=\"..\"/></Router>",
			)
			.await
			.unwrap();
		let resolved =
			resolve_in_repo_store(repo_store, "apps/site/main.bsx".to_string())
				.await
				.unwrap();
		resolved.entry_name.xpect_eq("site/main.bsx");
		#[cfg(not(target_arch = "wasm32"))]
		resolved.watch_dir.xpect_none();
		resolved
			.repo_store
			.exists(&SmolPath::from("site/main.bsx"))
			.await
			.unwrap()
			.xpect_true();
	}

	/// A root-level entry declaring a root above a store with no parent
	/// universe fails loudly naming the mis-publish.
	#[beet::test]
	async fn self_rooted_escape_fails_loudly() {
		let repo_store = BlobStore::temp();
		repo_store
			.insert(
				&SmolPath::from("main.bsx"),
				"<Router><RepoRoot src=\"../..\"/></Router>",
			)
			.await
			.unwrap();
		resolve_in_repo_store(repo_store, "main.bsx".to_string())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("mis-published");
	}

	/// The binary path and the command path share [`resolve_in_repo_store`], so the
	/// same inputs resolve an identical `(store, entry_name)`: here the
	/// command-shaped `resolve_main` against the binary-shaped store + name
	/// pair.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet::test]
	async fn both_paths_resolve_identically() {
		let tmp = TempDir::new().unwrap();
		fs_ext::write(
			tmp.path().join("main.bsx"),
			"<Router><RepoRoot src=\".\"/></Router>",
		)
		.unwrap();
		let by_path = resolve_main(None, tmp.path().to_string_lossy().as_ref())
			.await
			.unwrap();
		let by_store = resolve_in_repo_store(
			resolve_repo_store(None, tmp.path().clone()).unwrap(),
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

	/// Every `--watch` rebuild fires a fresh [`Ready`] on its fresh entry root,
	/// so a rebuilt tree boots exactly as the first one did: nothing is retained
	/// between builds.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet::test]
	async fn rebuild_fires_ready_every_time() {
		let repo_store = BlobStore::temp();
		repo_store
			.insert(&SmolPath::from("main.bsx"), "<Router/>")
			.await
			.unwrap();
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let readies = Store::new(0);
		let recorder = readies.clone();
		// the entry root is the one marked `TemplatesLoaded`.
		world.add_observer(
			move |ev: On<Ready>, entries: Query<(), With<TemplatesLoaded>>| {
				if entries.contains(ev.entity) {
					recorder.set(recorder.get() + 1);
				}
			},
		);
		let formats = world.get_resource_or_init::<TemplateFormats>().clone();
		world
			.run_async_local_then(move |world| async move {
				for _ in 0..2 {
					rebuild_watched(
						&world,
						repo_store.clone(),
						"main.bsx".to_string(),
						formats.clone(),
					)
					.await
					.unwrap();
				}
			})
			.await;
		readies.get().xpect_eq(2);
	}
}
