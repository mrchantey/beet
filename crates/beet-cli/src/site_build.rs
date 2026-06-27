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

/// Drive the async runtime until an entry root is marked [`TemplatesLoaded`], so a
/// build-then-serve path (the wasm Worker, a wasm one-shot) never serves before the
/// entry's templates registered.
///
/// [`AsyncRunner::settle_async_tasks`] settles the async runtime, but it can return
/// inside a synchronous-only window before the `<TemplateDir>`/`<RoutesDir>` scan
/// spawns its follow-up task, so idle alone is not readiness. This loops settle +
/// the explicit `TemplatesLoaded` check, ticking between, until the marker is present
/// or the safety cap is hit (so a never-loading entry returns rather than hanging).
///
/// The native run loop ticks naturally and `BootOnLoad` waits on the load itself, so
/// only the build-then-serve drivers need this explicit gate.
pub async fn settle_until_templates_loaded(world: &mut World) {
	// each iteration is a full settle; the cap guards a never-loading entry.
	const MAX_ITERS: usize = 64;
	for _ in 0..MAX_ITERS {
		AsyncRunner::settle_async_tasks(world).await;
		if world
			.query_filtered::<(), With<TemplatesLoaded>>()
			.iter(world)
			.next()
			.is_some()
		{
			return;
		}
		AsyncRunner::tick().await;
	}
	error!(
		"templates did not load within {MAX_ITERS} settle iterations; serving anyway"
	);
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

	/// The readiness gate settles and returns once the entry root carries
	/// [`TemplatesLoaded`] (which `build_entry_root` inserts), the signal a
	/// build-then-serve driver waits on before serving.
	#[beet::test]
	async fn gate_settles_on_templates_loaded() {
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
		// returns rather than hanging, the root being already marked loaded.
		settle_until_templates_loaded(&mut world).await;
	}
}
