//! The cross-platform entry resolution + build core shared by the native `beet`
//! binary, the wasm Worker entry, and the `check`/`serve`/`export-static`
//! commands.
//!
//! An entry load splits into resolution ([`resolve_main`]: the store + the entry
//! document name within it, honouring `--store` and the entry's own
//! `<StoreRoot src>`), a world-free async read ([`read_sources`]: the
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
	store: &BlobStore,
	entry_name: &str,
) -> Result<EntryPrescan> {
	store
		.get_media(&SmolPath::from(entry_name))
		.await?
		.xmap(|entry| EntryPrescan::parse(&entry))
}

/// Build the entry's store, honouring its own `<StoreRoot src>` declaration:
/// the root widens to `dir/src` (cleaned) and the entry name becomes the entry
/// path relative to it. Without a declaration the store roots at the entry's
/// own directory. Returns `(store, entry_name, root_dir)`; every local entry
/// load (the binary, `serve`/`check`/`export-static`) resolves through this so
/// an entry's declared root applies everywhere.
async fn widen_store_root(
	store_uri: Option<&StoreUri>,
	dir: AbsPathBuf,
	entry_name: String,
) -> Result<(BlobStore, String, AbsPathBuf, EntryPrescan)> {
	let store = resolve_store(store_uri, dir.clone())?;
	let prescan = read_prescan(&store, &entry_name).await?;
	let Some(src) = prescan.store_root.clone() else {
		return Ok((store, entry_name, dir, prescan));
	};
	let root = dir.join(src.as_str());
	let entry_name = dir
		.join(&entry_name)
		.strip_prefix(&root)
		.ok()
		.and_then(|rel| rel.to_str())
		.map(str::to_string)
		.ok_or_else(|| {
			bevyhow!(
				"entry `{dir}/{entry_name}` is not under its declared \
				`<StoreRoot src=\"{src}\"/>` (`{root}`)"
			)
		})?;
	// the widened store holds the same entry document, so its pre-scan is the one
	// already read: entry resolution parses the entry exactly once.
	Ok((
		resolve_store(store_uri, root.clone())?,
		entry_name,
		root,
		prescan,
	))
}

/// A resolved entry: its store, the entry document name within it, and the local
/// dir to watch for dev live reload (`None` for a self-rooted store, and always
/// `None` on wasm, where there is no fs-watcher backend).
pub struct ResolvedEntry {
	pub store: BlobStore,
	pub entry_name: String,
	/// The entry document's declarations, read by the same pass that resolved the
	/// store, so the load never re-parses it.
	pub prescan: EntryPrescan,
	#[cfg(not(target_arch = "wasm32"))]
	pub watch_dir: Option<AbsPathBuf>,
}

/// Resolve an explicit entry path (the binary's `--main`, a command's `<entry>`
/// positional): a path with an extension names the entry file itself, anything
/// else is a directory probed for the first [`ENTRY_NAMES`] match. Either way
/// the entry may widen its own store root with a `<StoreRoot src>` declaration
/// (see [`widen_store_root`]), and the `--store` param picks the backend.
pub async fn resolve_main(
	store_uri: Option<&StoreUri>,
	main: &str,
) -> Result<ResolvedEntry> {
	let path = AbsPathBuf::new(main)?;
	let (dir, entry_name) = if path.extension().is_some() {
		// an entry file: its parent is the initial root
		let dir = path.parent().ok_or_else(|| {
			bevyhow!("entry `{path}` has no parent directory")
		})?;
		let entry_name = path
			.file_name()
			.and_then(|name| name.to_str())
			.ok_or_else(|| bevyhow!("entry `{path}` has no file name"))?
			.to_string();
		(dir, entry_name)
	} else {
		// a directory: probe it for an entry document
		let store = resolve_store(store_uri, path.clone())?;
		let entry_name = probe_entry_names(&store).await?.ok_or_else(|| {
			bevyhow!(
				"no entry document found in `{path}`: looked for {ENTRY_NAMES:?}. \
				Create one, or name the entry file itself."
			)
		})?;
		(path, entry_name)
	};
	resolve_widened(store_uri, dir, entry_name).await
}

/// [`widen_store_root`] into a [`ResolvedEntry`], live reload watching the
/// resolved root.
pub async fn resolve_widened(
	store_uri: Option<&StoreUri>,
	dir: AbsPathBuf,
	entry_name: String,
) -> Result<ResolvedEntry> {
	let (store, entry_name, _root, prescan) =
		widen_store_root(store_uri, dir, entry_name).await?;
	Ok(ResolvedEntry {
		store,
		entry_name,
		prescan,
		#[cfg(not(target_arch = "wasm32"))]
		watch_dir: Some(_root),
	})
}

/// The first [`ENTRY_NAMES`] match at the store's root, if any.
pub async fn probe_entry_names(store: &BlobStore) -> Result<Option<String>> {
	for name in ENTRY_NAMES {
		if store.exists(&SmolPath::from(*name)).await? {
			return Ok(Some(name.to_string()));
		}
	}
	Ok(None)
}

/// Build the [`BlobStore`] a `--store` [`StoreUri`] names, defaulting to a
/// filesystem store rooted at `dir` (the resolved entry directory). Shared by
/// the binary's entry resolution (the launch config's `--store`) and the
/// `check`/`serve`/`export-static` commands (each command's own `--store`
/// param) so every entry load is store-driven rather than filesystem-bound.
pub fn resolve_store(
	store_uri: Option<&StoreUri>,
	dir: AbsPathBuf,
) -> Result<BlobStore> {
	BlobStore::from_uri(store_uri.unwrap_or(&StoreUri::default()), dir)
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
	store: &BlobStore,
	formats: TemplateFormats,
	entry_name: impl Into<String>,
	prescan: EntryPrescan,
) -> Result<EntrySources> {
	let entry_name = entry_name.into();
	let entry = store
		.get_media(&SmolPath::from(entry_name.as_str()))
		.await?;
	// a markup entry may declare `<TemplateDir>`s naming template directories; read
	// each so they register before the entry parses (so entry-level tags resolve). A
	// non-markup (serde) entry declares none.
	let mut template_sources = Vec::new();
	for dir in &prescan.template_dirs {
		template_sources
			.extend(TemplateDir::read_sources(store, dir, &formats).await?);
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
/// root. `run` is this loader's declaration that the entry should run itself (see
/// [`Ready::run`]); a render-only command leaves it `false` and the entry's
/// `CallOnReady` verbs stay dormant. Registers the entry's declared template sources
/// *before* parsing the entry (so its own tags resolve), then marks the root
/// [`TemplatesLoaded`]. The synchronous world-mutating tail of an entry load;
/// returns the root entity.
pub fn build_root(
	world: &mut World,
	store: BlobStore,
	sources: EntrySources,
	run: bool,
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
	// the site store on the root: descendants resolve it by ancestry. `TemplatesLoaded`
	// marks the entry-level templates registered (the readiness signal a wasm Worker
	// waits on before serving).
	let mut root_entity = world.entity_mut(root);
	root_entity.insert((extra, store, TemplatesLoaded));
	match run {
		true => root_entity.insert_template_run(template),
		false => root_entity.insert_template(template),
	}
	.map_err(|err| bevyhow!("failed to load entry `{entry_name}`: {err}"))?;
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
/// The build is dormant: this driver serves each request itself, so the entry's
/// declared servers must not start.
#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
pub(crate) async fn build_entry_owned(
	world: &mut World,
	store: BlobStore,
	entry_name: String,
) -> Result<Entity> {
	let formats = world.get_resource_or_init::<TemplateFormats>().clone();
	let prescan = read_prescan(&store, &entry_name).await?;
	let sources = read_sources(&store, formats, entry_name, prescan).await?;
	let root = build_root(world, store, sources, false, ())?;
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
	store: BlobStore,
	entry_name: String,
	formats: TemplateFormats,
) -> Result {
	let prescan = read_prescan(&store, &entry_name).await?;
	let sources =
		read_sources(&store, formats, entry_name.clone(), prescan).await?;
	// recompute the structural source set from the current content, so a
	// `<Template src>` include added by this very edit is structural on the next
	// one without a restart.
	let structural = entry_source_paths(&store, &entry_name).await;
	world
		.with(move |world: &mut World| -> Result {
			if let Some(mut reloader) =
				world.get_resource_mut::<EntryReloader>()
			{
				reloader.set_sources(structural);
			}
			// the entry's own dir, watched for edits to the entry doc / its includes;
			// computed before `build_root` consumes `store`.
			let entry_watch = WatchDir::for_entry(&store, &entry_name);
			// tear down the previous entry scene so servers close and sockets drop
			// before the fresh tree binds (a no-op on the first build).
			BeetSceneRoot::despawn_all(world);
			// a rebuild retains nothing from the last one: it simply declares `run`
			// again, so the fresh tree's servers rebind.
			let root = build_root(
				world,
				store,
				sources,
				true,
				(BeetSceneRoot, LiveReload::new()),
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
	store: &BlobStore,
	entry_name: &str,
) -> HashSet<SmolPath> {
	let mut seen = HashSet::default();
	let mut stack = vec![SmolPath::from(entry_name)];
	while let Some(path) = stack.pop() {
		if !seen.insert(path.clone()) {
			continue;
		}
		let Ok(media) = store.get_media(&path).await else {
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
/// builds onto a storeless ([`BlobStore::temp`]) root, so the same
/// [`build_root`] core runs as the store-backed native path. `run` declares the
/// build as for [`build_root`].
pub fn build_from_bsx(
	world: &mut World,
	formats: TemplateFormats,
	entry_name: impl Into<String>,
	bsx: impl Into<String>,
	run: bool,
) -> Result<Entity> {
	let entry = MediaBytes::new_bsx(bsx.into());
	let sources = EntrySources {
		entry_name: entry_name.into(),
		prescan: EntryPrescan::parse(&entry)?,
		entry,
		template_sources: Vec::new(),
		formats,
	};
	build_root(world, BlobStore::temp(), sources, run, ())
}

#[cfg(test)]
mod test {
	use super::*;

	/// The shared core builds an entry from any store: an in-memory store here, so
	/// it runs storage-agnostic (on wasm too), no filesystem involved. The entry's
	/// `<DefaultAppRoutes/>` lands on the built router root.
	#[beet::test]
	async fn builds_an_entry_from_an_in_memory_store() {
		let store = BlobStore::temp();
		store
			.insert(
				&SmolPath::from("main.bsx"),
				"<Router><DefaultAppRoutes/></Router>",
			)
			.await
			.unwrap();
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let formats = world.get_resource_or_init::<TemplateFormats>().clone();
		let prescan = read_prescan(&store, "main.bsx").await.unwrap();
		let sources = read_sources(&store, formats, "main.bsx", prescan)
			.await
			.unwrap();
		let root = build_root(&mut world, store, sources, false, ()).unwrap();
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
		let store = BlobStore::temp();
		store
			.insert(&SmolPath::from("main.bsx"), "<Router/>")
			.await
			.unwrap();
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let formats = world.get_resource_or_init::<TemplateFormats>().clone();
		let prescan = read_prescan(&store, "main.bsx").await.unwrap();
		let sources = read_sources(&store, formats, "main.bsx", prescan)
			.await
			.unwrap();
		let root = build_root(&mut world, store, sources, false, ()).unwrap();
		world
			.entity(root)
			.contains::<TemplatesLoaded>()
			.xpect_true();
		// returns rather than hanging, nothing being pending on this entry.
		TemplatePending::settle_owned(&mut world).await;
	}

	/// The `--watch` rebuild declares `run` on every build, so a rebuilt tree
	/// boots exactly as the first one did. Nothing is retained between builds:
	/// the declaration is per-build and consumed by its own sweep.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet::test]
	async fn rebuild_declares_run_every_time() {
		let store = BlobStore::temp();
		store
			.insert(&SmolPath::from("main.bsx"), "<Router/>")
			.await
			.unwrap();
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let runs = Store::new(Vec::<bool>::new());
		let recorder = runs.clone();
		// the entry root is the one marked `TemplatesLoaded`.
		world.add_observer(
			move |ev: On<Ready>, entries: Query<(), With<TemplatesLoaded>>| {
				if entries.contains(ev.entity) {
					let mut all = recorder.get();
					all.push(ev.run);
					recorder.set(all);
				}
			},
		);
		let formats = world.get_resource_or_init::<TemplateFormats>().clone();
		world
			.run_async_local_then(move |world| async move {
				for _ in 0..2 {
					rebuild_watched(
						&world,
						store.clone(),
						"main.bsx".to_string(),
						formats.clone(),
					)
					.await
					.unwrap();
				}
			})
			.await;
		runs.get().xpect_eq(vec![true, true]);
	}
}
