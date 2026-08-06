//! The cross-platform entry resolution + build core shared by the native `beet`
//! binary, the wasm Worker entry, and the `check`/`serve`/`export-static`
//! commands.
//!
//! An entry load splits into resolution ([`resolve_main`]: the store + the entry
//! document name within it, honouring `--store` and the entry's own
//! `<StoreRoot src>`), a world-free async read ([`read_entry_sources`]: the
//! entry document and the templates under its declared `<TemplateDir>`s, through the
//! [`BlobStore`]) and a synchronous world build ([`build_entry_root`]: register the
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

/// The entry's declared `<StoreRoot src>`, if any: a registry-free pre-scan of
/// the raw entry document, run before the store builds so the declaration can
/// widen the store root (markup only; a serde entry declares none).
async fn read_store_root(
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
async fn widen_store_root(
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

/// A resolved entry: its store, the entry document name within it, and the local
/// dir to watch for dev live reload (`None` for a self-rooted store, and always
/// `None` on wasm, where there is no fs-watcher backend).
pub struct ResolvedEntry {
	pub store: BlobStore,
	pub entry_name: String,
	#[cfg(not(target_arch = "wasm32"))]
	pub watch_dir: Option<AbsPathBuf>,
}

/// Resolve an explicit entry path (the binary's `--main`, a command's `<entry>`
/// positional): a path with an extension names the entry file itself, anything
/// else is a directory probed for the first [`ENTRY_NAMES`] match. Either way
/// the entry may widen its own store root with a `<StoreRoot src>` declaration
/// (see [`widen_store_root`]), and the `--store` param picks the backend.
pub async fn resolve_main(
	params: &MultiMap<SmolStr, SmolStr>,
	main: &str,
) -> Result<ResolvedEntry> {
	let path = AbsPathBuf::new(main)?;
	let (dir, entry_name) = if path.extension().is_some() {
		// an entry file: its parent is the initial root
		let dir = path
			.parent()
			.ok_or_else(|| bevyhow!("entry `{path}` has no parent directory"))?;
		let entry_name = path
			.file_name()
			.and_then(|name| name.to_str())
			.ok_or_else(|| bevyhow!("entry `{path}` has no file name"))?
			.to_string();
		(dir, entry_name)
	} else {
		// a directory: probe it for an entry document
		let store = resolve_store(params, path.clone())?;
		let entry_name = probe_entry_names(&store).await?.ok_or_else(|| {
			bevyhow!(
				"no entry document found in `{path}`: looked for {ENTRY_NAMES:?}. \
				Create one, or name the entry file itself."
			)
		})?;
		(path, entry_name)
	};
	resolve_widened(params, dir, entry_name).await
}

/// [`widen_store_root`] into a [`ResolvedEntry`], live reload watching the
/// resolved root.
pub async fn resolve_widened(
	params: &MultiMap<SmolStr, SmolStr>,
	dir: AbsPathBuf,
	entry_name: String,
) -> Result<ResolvedEntry> {
	let (store, entry_name, _root) =
		widen_store_root(params, dir, entry_name).await?;
	Ok(ResolvedEntry {
		store,
		entry_name,
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

/// Whether the `--store` param selects a self-rooted backend (a bucket / browser
/// storage) needing no local directory or filesystem walk: `s3://<bucket>`,
/// `local-storage`, `indexed-db`. For these `--main` names the entry document
/// *within* the store (defaulting to an [`ENTRY_NAMES`] probe) and there is no
/// live-reload watch dir.
pub fn store_is_self_rooted(params: &MultiMap<SmolStr, SmolStr>) -> bool {
	params
		.get("store")
		.map(|kind| {
			kind.starts_with("s3://")
				|| matches!(kind.as_str(), "local-storage" | "indexed-db")
		})
		.unwrap_or(false)
}

/// Build the [`BlobStore`] selected by the `--store` param. Shared by the binary's
/// entry resolution and the `check`/`serve`/`export-static` commands so every
/// entry load is store-driven rather than filesystem-bound. Dir-rooted kinds root
/// at `dir` (the resolved entry directory); self-rooted kinds
/// ([`store_is_self_rooted`]) ignore it.
///
/// - `fs` (default): a filesystem store rooted at `dir`. Cross-platform, since
///   [`FsStore`] reads through `fs_ext` (the deno runner's fs globals on wasm).
/// - `memory`: a temporary in-memory store, only meaningful with an explicit
///   entry since it has no seeded entry to discover.
/// - `s3://<bucket>[?endpoint=<url>][&region=<region>]` (`aws_sdk`, native): the
///   bucket a deployed task serves from; see [`resolve_s3_store`].
/// - `local-storage` / `indexed-db` (wasm): browser storage.
///
/// An unknown kind errors with the supported list.
pub fn resolve_store(
	params: &MultiMap<SmolStr, SmolStr>,
	dir: AbsPathBuf,
) -> Result<BlobStore> {
	let kind = params.get("store").map(SmolStr::as_str).unwrap_or("fs");
	if let Some(rest) = kind.strip_prefix("s3://") {
		return resolve_s3_store(rest);
	}
	match kind {
		"fs" => BlobStore::new(FsStore::new(dir)).xok(),
		"memory" => BlobStore::temp().xok(),
		#[cfg(target_arch = "wasm32")]
		"local-storage" => BlobStore::new(LocalStorageStore::new("beet")).xok(),
		#[cfg(target_arch = "wasm32")]
		"indexed-db" => BlobStore::new(IndexedDbStore::new("beet")).xok(),
		#[cfg(not(target_arch = "wasm32"))]
		"local-storage" | "indexed-db" => bevybail!(
			"--store={kind} is browser storage, only available on wasm"
		),
		other => bevybail!(
			"unknown --store `{other}`, supported kinds: fs, memory, \
			s3://<bucket>[?endpoint=..][&region=..], local-storage (wasm), \
			indexed-db (wasm)"
		),
	}
}

/// Parse the `s3://<bucket>[?endpoint=<url>][&region=<region>]` store uri into an
/// [`S3Store`]-backed [`BlobStore`]. An `endpoint` (eg
/// `https://<account>.r2.cloudflarestorage.com`) switches onto an S3-compatible
/// service such as Cloudflare R2 with region `auto`, so one binary serves
/// identically on AWS S3 and R2; otherwise the region falls back to the SDK's
/// `AWS_REGION` convention, then `us-west-2`.
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
fn resolve_s3_store(rest: &str) -> Result<BlobStore> {
	let (bucket, query) = rest.split_once('?').unwrap_or((rest, ""));
	if bucket.is_empty() {
		bevybail!("--store=s3:// is missing a bucket name");
	}
	let mut endpoint = None;
	let mut region = None;
	for pair in query.split('&').filter(|pair| !pair.is_empty()) {
		match pair.split_once('=') {
			Some(("endpoint", value)) => endpoint = Some(value.to_string()),
			Some(("region", value)) => region = Some(value.to_string()),
			_ => bevybail!(
				"unknown --store=s3 query param `{pair}`, supported: endpoint, \
				region"
			),
		}
	}
	let store = match endpoint {
		Some(endpoint) => {
			info!("entry store: r2/s3 bucket `{bucket}` ({endpoint})");
			S3Store::new(bucket, region.unwrap_or_else(|| "auto".to_string()))
				.with_endpoint(endpoint)
		}
		None => {
			let region = region
				.or_else(|| env_ext::var("AWS_REGION").ok())
				.unwrap_or_else(|| "us-west-2".to_string());
			info!("entry store: s3 bucket `{bucket}` ({region})");
			S3Store::new(bucket, region)
		}
	};
	BlobStore::new(store).xok()
}

/// Without a compiled S3 backend the request errors with guidance rather than
/// degrading (the store concept is target-agnostic, only the backend is gated).
#[cfg(not(all(feature = "aws_sdk", not(target_arch = "wasm32"))))]
fn resolve_s3_store(_rest: &str) -> Result<BlobStore> {
	bevybail!(
		"--store=s3://.. requires a compiled S3 backend (enable the `aws_sdk` \
		feature, native only)"
	)
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
/// root (eg `DisableCallOnLoad` for a render-only build). Registers the entry's
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
	world
		.entity_mut(root)
		.insert((extra, store, TemplatesLoaded))
		.insert_template(template)
		.map_err(|err| {
			bevyhow!("failed to load entry `{entry_name}`: {err}")
		})?;
	world.flush();
	Ok(root)
}

/// Build an entry into an owned world and settle it to readiness: read the
/// sources through `store`, build the root carrying `extra`, then drive the
/// async runtime until every pending set drains
/// ([`TemplatePending::settle_owned`]), so `<RoutesDir>`/`<TemplateDir>` scans
/// land before the caller serves. The world-owning driver path (the wasm Worker,
/// a one-shot build); an in-app caller settles via [`TemplatePending::settle`]
/// instead. Returns the entry root.
pub(crate) async fn build_entry_owned(
	world: &mut World,
	store: BlobStore,
	entry_name: String,
	extra: impl Bundle,
) -> Result<Entity> {
	let formats = world.get_resource_or_init::<TemplateFormats>().clone();
	let sources = read_entry_sources(&store, formats, entry_name).await?;
	let root = build_entry_root(world, store, sources, extra)?;
	TemplatePending::settle_owned(world).await;
	Ok(root)
}

/// Rebuild the `--watch` entry into a fresh [`BeetSceneRoot`], the shared path the
/// initial build and every structural reload run: tear down the previous entry
/// scene via [`despawn_scene`] (servers close, sockets drop; a no-op on the first
/// build), re-read the sources through the store, and build a fresh root marked
/// [`BeetSceneRoot`] + [`LiveReload`] with its own entry [`WatchDir`]. The fresh
/// root's server children re-boot (rebinding their ports), so a browser's dropped
/// `/__client_io` socket reconnects and reloads into the new tree.
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
	let sources =
		read_entry_sources(&store, formats, entry_name.clone()).await?;
	// recompute the structural source set from the current content, so a
	// `<Template src>` include added by this very edit is structural on the next
	// one without a restart.
	let structural = entry_source_paths(&store, &entry_name).await;
	world
		.with(move |world: &mut World| -> Result {
			if let Some(mut reloader) = world.get_resource_mut::<EntryReloader>()
			{
				reloader.set_sources(structural);
			}
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
			build_entry_root(&mut world, store, sources, DisableCallOnLoad)
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
			build_entry_root(&mut world, store, sources, DisableCallOnLoad)
				.unwrap();
		world
			.entity(root)
			.contains::<TemplatesLoaded>()
			.xpect_true();
		// returns rather than hanging, nothing being pending on this entry.
		TemplatePending::settle_owned(&mut world).await;
	}
}
