//! The cross-platform entry build core shared by the native `beet` binary, the wasm
//! Worker entry, and the `check`/`export-static` commands.
//!
//! An entry load splits into a world-free async read ([`read_entry_sources`]: the
//! entry document and the templates under its declared `<TemplateDir>`s, through the
//! [`BlobStore`]) and a synchronous world build ([`build_entry_root`]: register the
//! templates, parse the entry, build it into a root carrying the store). The entry's
//! own template dirs are registered *before* the entry parses, so entry-level tags
//! (eg `<Styles/>`) resolve; the reactive `<TemplateDir>` observer covers everything
//! that loads later (route pages, library widgets). The same path runs on the native
//! async runtime and the single-threaded wasm Worker, so entry resolution comes from
//! an injected store rather than a filesystem walk.

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

/// The entry's declared `<StoreRoot src>`, if any: a registry-free pre-scan of
/// the raw entry document, run before the store builds so the declaration can
/// widen the store root (markup only; a serde entry declares none).
pub async fn read_store_root(
	store: &BlobStore,
	entry_name: &str,
) -> Result<Option<String>> {
	let entry = store.get_media(&SmolPath::from(entry_name)).await?;
	match entry.media_type() {
		MediaType::Bsx | MediaType::Html => {
			parse_document(entry.as_utf8()?, &BsxParseConfig::bsx())?
				.xmap(|nodes| StoreRoot::extract_root(&nodes))
				.xok()
		}
		_ => Ok(None),
	}
}

/// Build the entry's store, honouring its own `<StoreRoot src>` declaration:
/// the root widens to `dir/src` (cleaned) and the entry name becomes the entry
/// path relative to it. Without a declaration the store roots at the entry's
/// own directory. Returns `(store, entry_name, root_dir)`; every local entry
/// load (the binary, `serve`/`check`/`export-static`) resolves through this so
/// an entry's declared root applies everywhere.
pub async fn widen_store_root(
	params: &MultiMap<SmolStr, SmolStr>,
	dir: AbsPathBuf,
	entry_name: String,
) -> Result<(BlobStore, String, AbsPathBuf)> {
	let store = resolve_store(params, dir.clone())?;
	let Some(src) = read_store_root(&store, &entry_name).await? else {
		return Ok((store, entry_name, dir));
	};
	let root = dir.join(&src);
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
	Ok((resolve_store(params, root.clone())?, entry_name, root))
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

/// Build the [`BlobStore`] selected by the `--store` param, rooted at `dir`. Shared
/// by the binary's entry resolution and the `check`/`serve`/`export-static` commands
/// so every entry/site load is store-driven rather than filesystem-bound.
///
/// Cross-platform, since [`FsStore`] is cross-platform: it reads through `fs_ext`,
/// which routes to the deno runner's fs globals on wasm, so the wasm `beet` binary
/// resolves the same on-disk entry native does, no separate backend needed.
/// - `fs` (default): a filesystem store rooted at `dir`.
/// - `memory`: a temporary in-memory store.
///
/// `memory` is only meaningful with an explicit entry, since it has no seeded entry to
/// discover. An unknown kind errors with the supported list.
pub fn resolve_store(
	params: &MultiMap<SmolStr, SmolStr>,
	dir: AbsPathBuf,
) -> Result<BlobStore> {
	match params.get("store").map(SmolStr::as_str).unwrap_or("fs") {
		"fs" => BlobStore::new(FsStore::new(dir)).xok(),
		"memory" => BlobStore::temp().xok(),
		other => {
			bevybail!("unknown --store `{other}`, supported kinds: fs, memory")
		}
	}
}

/// The entry sources read from a store: the entry document bytes + name, the template
/// `(path, source)` pairs from the entry's declared `<TemplateDir>`s, and the formats
/// they register through. The world-free async read [`build_entry_root`] consumes.
pub struct EntrySources {
	entry_name: String,
	entry: MediaBytes,
	template_sources: Vec<(SmolPath, String)>,
	formats: TemplateFormats,
}

/// Read the entry document and the templates under its declared `<TemplateDir>`s
/// through `store`, awaited off the runtime (never blocked, so it runs on the
/// single-threaded Worker too). The caller reads `formats` from the world first,
/// since the read itself is world-free.
pub async fn read_entry_sources(
	store: &BlobStore,
	formats: TemplateFormats,
	entry_name: impl Into<String>,
) -> Result<EntrySources> {
	let entry_name = entry_name.into();
	let entry = store
		.get_media(&SmolPath::from(entry_name.as_str()))
		.await?;
	// a markup entry may declare `<TemplateDir>`s naming template directories; read
	// each so they register before the entry parses (so entry-level tags resolve). A
	// non-markup (serde) entry declares none.
	let template_sources = match entry.media_type() {
		MediaType::Bsx | MediaType::Html => {
			let nodes =
				parse_document(entry.as_utf8()?, &BsxParseConfig::bsx())?;
			let mut sources = Vec::new();
			for dir in TemplateDir::extract_dirs(&nodes) {
				sources.extend(
					TemplateDir::read_sources(store, &dir, &formats).await?,
				);
			}
			sources
		}
		_ => Vec::new(),
	};
	EntrySources {
		entry_name,
		entry,
		template_sources,
		formats,
	}
	.xok()
}

/// Build read [`EntrySources`] into a root carrying `store` (resolved by ancestry for
/// `<TemplateDir>`, `<RoutesDir>` and `<Template src>`), with `extra` riding onto the
/// root (eg `DisableBootOnLoad` for a render-only build). Registers the entry's
/// declared template sources *before* parsing the entry (so its own tags resolve),
/// then marks the root [`TemplatesLoaded`]. The synchronous world-mutating tail of an
/// entry load; returns the root entity.
pub fn build_entry_root(
	world: &mut World,
	store: BlobStore,
	sources: EntrySources,
	extra: impl Bundle,
) -> Result<Entity> {
	let EntrySources {
		entry_name,
		entry,
		template_sources,
		formats,
	} = sources;
	// registry-free pre-scan: spawn the entry's `<CrateCheck>`s before the tree
	// builds, so a check fires with its missing-feature list even when the tree
	// itself cannot build (eg its root tag is feature-gated out of this binary).
	if matches!(entry.media_type(), MediaType::Bsx | MediaType::Html)
		&& let Ok(text) = entry.as_utf8()
		&& let Ok(nodes) = parse_document(text, &BsxParseConfig::bsx())
	{
		for check in CrateCheck::extract_checks(&nodes) {
			world.spawn(check);
		}
		world.flush();
	}
	// the entry's own template dirs, registered before the entry parses so its
	// entry-level tags (eg `<Styles/>`) resolve. The reactive `<TemplateDir>` observer
	// re-registers them (plus any crate/route dirs) once the tree is built.
	TemplateDir::register_sources(world, &formats, template_sources)?;
	let template = EntryTemplate::from_bytes(world, &entry).map_err(|err| {
		bevyhow!("failed to parse entry `{entry_name}`: {err}")
	})?;
	// the site store on the root: descendants resolve it by ancestry. `TemplatesLoaded`
	// marks the entry-level templates registered (the readiness signal a wasm Worker
	// waits on before serving).
	let root = world.spawn((extra, store, TemplatesLoaded)).id();
	world
		.entity_mut(root)
		.insert_template(template)
		.map_err(|err| {
			bevyhow!("failed to load entry `{entry_name}`: {err}")
		})?;
	world.flush();
	Ok(root)
}

/// Rebuild the `--watch` entry into a fresh [`BeetSceneRoot`], the shared path the
/// initial build and every structural reload run: tear down the previous entry
/// scene via [`despawn_scene`] (servers close, sockets drop; a no-op on the first
/// build), re-read the sources through the store, and build a fresh root marked
/// [`BeetSceneRoot`] + [`LiveReload`] with its own entry [`WatchDir`]. The fresh
/// root's `BootOnLoad` re-boots its servers (rebinding their ports), so a browser's
/// dropped `/__client_io` socket reconnects and reloads into the new tree.
///
/// The [`EntryReloader`] resource (installed once) survives the teardown and drives
/// this on a change to the entry document or an included `<Template src>`.
#[cfg(not(target_arch = "wasm32"))]
pub async fn rebuild_watched_entry(
	world: &AsyncWorld,
	store: BlobStore,
	entry_name: String,
	formats: TemplateFormats,
) -> Result {
	let sources = read_entry_sources(&store, formats, entry_name.clone()).await?;
	world
		.with(move |world: &mut World| -> Result {
			// the entry's own dir, watched for edits to the entry doc / its includes;
			// computed before `build_entry_root` consumes `store`.
			let entry_watch = WatchDir::for_entry(&store, &entry_name);
			// tear down the previous entry scene so servers close and sockets drop
			// before the fresh tree binds (a no-op on the first build).
			despawn_scene(world);
			let root = build_entry_root(
				world,
				store,
				sources,
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
pub async fn entry_source_paths(
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
		if !matches!(media.media_type(), MediaType::Bsx | MediaType::Html) {
			continue;
		}
		let Ok(text) = media.as_utf8() else {
			continue;
		};
		let Ok(nodes) = parse_document(text, &BsxParseConfig::bsx()) else {
			continue;
		};
		stack.extend(
			extract_template_srcs(&nodes)
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
/// [`build_entry_root`] core runs as the store-backed native path. `extra` rides
/// onto the root as for [`build_entry_root`].
pub fn build_entry_from_bsx(
	world: &mut World,
	formats: TemplateFormats,
	entry_name: impl Into<String>,
	bsx: impl Into<String>,
	extra: impl Bundle,
) -> Result<Entity> {
	let sources = EntrySources {
		entry_name: entry_name.into(),
		entry: MediaBytes::new_bsx(bsx.into()),
		template_sources: Vec::new(),
		formats,
	};
	build_entry_root(world, BlobStore::temp(), sources, extra)
}

/// Drive the async runtime until a build-then-serve entry is fully ready to serve: its
/// `<TemplateDir>` templates registered, its `<RoutesDir>` routes discovered, and any
/// `<Template src>` includes resolved. A build-then-serve driver (the wasm Worker, a
/// wasm one-shot) owns the world, so it settles + ticks and re-checks readiness itself
/// rather than yielding to an outer loop (the in-app [`RoutesDir::settle_all`] path).
///
/// Readiness is entity state, not idle. [`build_entry_root`] marks the root
/// [`TemplatesLoaded`] *synchronously*, and the route/template scans only spawn their
/// follow-up tasks a few async ticks after the insert, so "the root is loaded" (or a
/// single idle window) can precede any route existing, the cause of intermittent 404s
/// on a multi-route site served from a Worker. This instead mirrors
/// [`RoutesDir::settle_all`]'s conditions: a `<RoutesDir>` with no composed
/// [`BlobStore`] is still discovering, a `<TemplateDir>` without [`TemplatesLoaded`] is
/// still registering, and a non-empty [`TemplatePending`] is an unresolved include. It
/// loops settle + check, ticking between, until nothing is pending or the safety cap is
/// hit (so a never-loading entry returns rather than hanging).
///
/// The native run loop ticks naturally and `BootOnLoad` waits on the load itself, so
/// only the build-then-serve drivers need this explicit gate.
pub async fn settle_until_ready(world: &mut World) {
	// each iteration is a full settle; the cap guards a never-loading entry.
	const MAX_ITERS: usize = 64;
	for _ in 0..MAX_ITERS {
		AsyncRunner::settle_async_tasks(world).await;
		if entry_pending(world) == 0 {
			return;
		}
		AsyncRunner::tick().await;
	}
	error!(
		"entry did not settle within {MAX_ITERS} iterations; serving anyway \
		(routes may be incomplete)"
	);
}

/// The count of still-loading entry dependencies, mirroring [`RoutesDir::settle_all`]:
/// routes still discovering (no scoped [`BlobStore`] composed yet), template dirs still
/// registering (not yet [`TemplatesLoaded`]), and unresolved `<Template src>` includes
/// (a non-empty [`TemplatePending`]). Zero means the entry is ready to serve.
fn entry_pending(world: &mut World) -> usize {
	let discovering_routes = world
		.query_filtered::<(), (With<RoutesDir>, Without<BlobStore>)>()
		.iter(world)
		.count();
	let registering_templates = world
		.query_filtered::<(), (With<TemplateDir>, Without<TemplatesLoaded>)>()
		.iter(world)
		.count();
	let unresolved_includes = world
		.query::<&TemplatePending>()
		.iter(world)
		.filter(|pending| !pending.is_empty())
		.count();
	discovering_routes + registering_templates + unresolved_includes
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
		let sources = read_entry_sources(&store, formats, "main.bsx")
			.await
			.unwrap();
		let root =
			build_entry_root(&mut world, store, sources, DisableBootOnLoad)
				.unwrap();
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
	/// `build_entry_root` returns; the gate must return rather than hang.
	#[beet::test]
	async fn gate_settles_when_ready() {
		let store = BlobStore::temp();
		store
			.insert(&SmolPath::from("main.bsx"), "<Router/>")
			.await
			.unwrap();
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let formats = world.get_resource_or_init::<TemplateFormats>().clone();
		let sources = read_entry_sources(&store, formats, "main.bsx")
			.await
			.unwrap();
		let root =
			build_entry_root(&mut world, store, sources, DisableBootOnLoad)
				.unwrap();
		world
			.entity(root)
			.contains::<TemplatesLoaded>()
			.xpect_true();
		// returns rather than hanging, nothing being pending on this entry.
		settle_until_ready(&mut world).await;
	}
}
