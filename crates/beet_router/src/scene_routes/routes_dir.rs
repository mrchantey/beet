//! Runtime route discovery: a directory of content files becomes routes at
//! spawn time, no codegen.
//!
//! Inserting a [`RoutesDir`] (eg from a `main.bsx` entry via
//! `<RoutesDir src="routes"/>`) triggers [`RoutesDir::spawn_on_insert`]: the
//! nearest ancestor [`BlobStore`] (the repo store composed on the loaded root) is
//! scoped to `src` and listed, and each content file
//! (`.md`/`.mdx`/`.bsx`/`.html`) spawns a [`BlobScene`] route child served through
//! the shared media-parse pipeline. The scoped [`BlobStore`] is composed onto the
//! [`RoutesDir`] entity so the routes read their bytes from it, and markdown
//! frontmatter is read at scan time into [`ArticleMeta`] so navigation (eg
//! [`RouteSidebar`](crate::prelude::RouteSidebar)) knows every page's title/order
//! without visiting it. Discovery is store-backed, so it reads identically from
//! the local filesystem in dev and from S3 in a deployed task.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Spawns one [`BlobScene`] route child per content file under `src`,
/// discovered at spawn time (see the module docs).
///
/// Route paths mirror the file tree: `docs/intro.md` serves at `docs/intro`,
/// and an `index.*` file collapses to its directory (`docs/index.md` serves at
/// `docs`). Frontmatter then has the last word on both the url and the ordering:
/// a `slug` renames the final segment and a leading `<number>-` on the filename
/// sets the nav order, so `blog/1-full-stack-bevy.md` declaring
/// `slug = "full-stack-bevy"` reads first and serves at `blog/full-stack-bevy`.
/// Add a [`PathPartial`] alongside to prefix every discovered route.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RoutesDir {
	/// The content directory, relative to the nearest ancestor [`BlobStore`].
	pub src: String,
}

/// The content file extensions served as [`BlobScene`] routes.
const CONTENT_EXTENSIONS: &[&str] = &["md", "mdx", "markdown", "html", "bsx"];

impl RoutesDir {
	/// Discover routes under `src`, relative to the nearest ancestor [`BlobStore`].
	pub fn new(src: impl Into<String>) -> Self { Self { src: src.into() } }

	/// Observer: scan the [`RoutesDir`] store and spawn its routes (see the module docs).
	///
	/// The scan is store I/O (the filesystem in dev, S3 in a deployed task, R2 in a
	/// Worker), so it runs as an [`AsyncEntity`] task rather than blocking the runtime
	/// (which is single-threaded on wasm). The nearest ancestor [`BlobStore`] (the site
	/// store composed on the loaded root) is resolved *inside* that task, where the
	/// whole tree is already built, so the ancestor link is reliably present; a
	/// store-less app is an error (never an implicit filesystem store, which has none
	/// on wasm).
	///
	/// The route children appear a few async ticks after the insert, so the scan
	/// parks a [`PendingGuard`] on the build root (or on this entity outside a
	/// build), deferring [`Ready`] until the routes are spawned. So a load
	/// verb (`CallOnReady`) under the entry root only fans the request out once
	/// every discovered route exists, exactly as the asset / scene deferrals gate
	/// it, and a settle ([`TemplatePending::settle`]) waits on the same set
	/// wherever the dir was inserted.
	pub fn spawn_on_insert(
		ev: On<Insert, RoutesDir>,
		dirs: Query<&RoutesDir>,
		build_root: Option<Res<TemplateBuildRoot>>,
		mut commands: Commands,
	) -> Result {
		let entity = ev.entity;
		let src = SmolPath::from(dirs.get(entity)?.src.as_str());
		let root = build_root.map(|root| **root);
		// one queued command parks the guard (ahead of the build's synchronous
		// drain, like the scene-ready gate) and spawns the scan task holding it,
		// so however the task ends the guard resolves.
		//
		// `run_async_local` (not `run_async`): the scan is bridge-heavy (resolve the
		// ancestor store, then compose it + spawn routes back on the world), and the async
		// bridge only *guarantees* a bridge poll completes when the task runs on the
		// runtime's local executor. A `bevy_multithreaded` build's `spawn` would run it on
		// an `IoTaskPool` worker thread, whose bridge poll can perpetually miss the
		// main-thread world-scope window and stall the scan. Pinning it local keeps
		// discovery deterministic on every target.
		commands.queue(move |world: &mut World| {
			let guard = TemplatePending::park_on(
				world,
				root.unwrap_or(entity),
				PendingKind::Passive,
				format!("<RoutesDir src=\"{src}\"> scan"),
			);
			let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
				// the dir despawned before the command ran: the dropped guard
				// resolves through the sweep.
				return;
			};
			// off the async runtime: resolve the nearest ancestor store + scope it
			// to `src`, await the content scan, then compose the scoped store onto
			// the entity, spawn the route children, and flush so the route-tree
			// observers settle against the whole hierarchy.
			entity_mut.run_async_local(
				async move |dir: AsyncEntity| -> Result {
					let store = dir
						.with_state::<AncestorQuery<&BlobStore>, Result<BlobStore>>(
							|entity, stores| {
								stores.get(entity).map(BlobStore::clone)
							},
						)
						.await??
						.with_subdir(src);
					let specs = Self::discover_routes(&store).await?;
					dir.world()
						.with(move |world| {
							// watch the discovered routes dir for live reload (keyed to
							// its base store); inert on a non-fs store / on wasm.
							let watch = WatchDir::from_store(&store);
							let mut entity_mut = world.entity_mut(entity);
							entity_mut.insert(store);
							if let Some(watch) = watch {
								entity_mut.insert(watch);
							}
							// every valid route still spawns, so one bad slug does
							// not take the site down with it; the failures are
							// reported together once the guard has resolved.
							let mut failures = Vec::new();
							for spec in specs {
								if let Err(err) =
									Self::spawn_route_spec(world, entity, spec)
								{
									failures.push(err.to_string());
								}
							}
							world.flush();
							// routes are spawned: resolve, draining the root's set so
							// the deferred `Ready` fires.
							guard.resolve(world);
							failures
						})
						.await
						.xmap(|failures| match failures.is_empty() {
							true => Ok(()),
							false => Err(bevyhow!(
								"{} discovered route(s) failed to spawn:\n{}",
								failures.len(),
								failures.join("\n")
							)),
						})
				},
			);
		});
		Ok(())
	}

	/// Spawn one discovered content file as a [`BlobScene`] route child of `parent`.
	///
	/// The metadata is resolved first because it has the last word on the url: a
	/// `slug` renames the filename-derived final segment. A BSX page's metadata
	/// resolves here rather than in the scan because reflect-building its spread
	/// needs the world's type registry.
	fn spawn_route_spec(
		world: &mut World,
		parent: Entity,
		spec: RouteSpec,
	) -> Result {
		let meta = spec
			.meta
			.resolve(world)
			.map(|meta| meta.with_file_defaults(&spec.store_path));
		let route_path = Self::route_path_for(&spec.store_path, meta.as_ref())?;
		let mut route_entity = world.spawn((
			ChildOf(parent),
			route::new(route_path.as_str(), BlobScene::new(spec.store_path)),
			HttpMethod::Get,
			ExportStrategy::Static,
			// a discovered content file is a user-facing page, so it carries
			// `PageRoute` and appears in the nav, like its codegen blob equivalent.
			PageRoute,
		));
		// scan-time page metadata, so navigation knows titles/order up front
		if let Some(meta) = meta {
			route_entity.insert(meta);
		}
		Ok(())
	}

	/// List the store's content files and read each one's declared metadata,
	/// returning route specs in lexical path order so zero-padded routes (eg slides
	/// `01..20`) spawn in sequence, giving a deterministic [`RouteTree`] child order.
	///
	/// This half is the store I/O; what the bytes MEAN settles at spawn time (see
	/// [`spawn_route_spec`](Self::spawn_route_spec)), which is also where a `slug`
	/// renames the route path — after the sort, so it cannot reshuffle the order.
	async fn discover_routes(store: &BlobStore) -> Result<Vec<RouteSpec>> {
		let mut paths = store.list().await?;
		paths.sort();
		paths
			.into_iter()
			.filter(|path| Self::is_content(path))
			.map(async |path| -> Result<RouteSpec> {
				Ok(RouteSpec {
					meta: Self::scan_meta(store, &path).await,
					store_path: path,
				})
			})
			.xmap(async_ext::try_join_all)
			.await
	}

	/// Whether `path`'s extension marks it as a servable content file.
	fn is_content(path: &SmolPath) -> bool {
		path.extension()
			.is_some_and(|ext| CONTENT_EXTENSIONS.contains(&ext))
	}

	/// The route path a content file serves at before frontmatter has its say:
	/// the extension is dropped and a trailing `index` collapses to its
	/// directory, eg `docs/index.md` -> `docs`.
	///
	/// A pure function of the filename, shared with the codegen collection scan
	/// so a route path never depends on which scan found the file. The
	/// frontmatter `slug` override is applied on top by the caller, which holds
	/// the parsed [`ArticleMeta`] (see [`ArticleMeta::apply_slug`]).
	pub(crate) fn route_path_of(rel: &SmolPath) -> SmolPath {
		let mut segments = rel.segments();
		if let (Some(stem), Some(last)) = (rel.file_stem(), segments.last_mut())
		{
			*last = stem;
		}
		if segments.last() == Some(&"index") {
			segments.pop();
		}
		SmolPath::from_segments(&segments)
	}

	/// The url a content file serves at: its filename-derived path
	/// ([`route_path_of`](Self::route_path_of)) with a declared `slug` renaming
	/// the final segment.
	///
	/// # Errors
	/// Errors when an `index` file declares a slug. Such a file collapses into
	/// its directory, so the segment a slug would rename belongs to the
	/// DIRECTORY, not the page: `blog/index.md` with `slug = "journal"` would
	/// serve at `/journal` while every sibling post stayed under `/blog`.
	fn route_path_for(
		store_path: &SmolPath,
		meta: Option<&ArticleMeta>,
	) -> Result<SmolPath> {
		let route_path = Self::route_path_of(store_path);
		let Some(meta) = meta.filter(|meta| meta.slug.is_some()) else {
			return Ok(route_path);
		};
		if store_path.file_stem() == Some("index") {
			bevybail!(
				"`{store_path}` declares a slug, but an index file collapses into its directory, \
				so it has no page segment of its own to rename"
			);
		}
		meta.apply_slug(&route_path)
	}

	/// Read a content file's declared page metadata through the store: markdown
	/// frontmatter, or the root spreads of a BSX document. Any read/parse failure
	/// yields [`RouteSpecMeta::None`], since a page without metadata is a page.
	async fn scan_meta(store: &BlobStore, path: &SmolPath) -> RouteSpecMeta {
		let Some(source) = store
			.get(path)
			.await
			.ok()
			.and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
		else {
			return RouteSpecMeta::None;
		};
		match path.extension() {
			#[cfg(feature = "markdown_parser")]
			Some("md" | "mdx" | "markdown") => ArticleMeta::from_markdown(&source)
				.map(RouteSpecMeta::Article)
				.unwrap_or(RouteSpecMeta::None),
			Some("bsx") => BsxNode::parse_document(&source, &default())
				.map(RouteSpecMeta::Bsx)
				.unwrap_or(RouteSpecMeta::None),
			_ => RouteSpecMeta::None,
		}
	}
}

/// A discovered content file: the store path its bytes load from, and the page
/// metadata that file declares.
struct RouteSpec {
	store_path: SmolPath,
	meta: RouteSpecMeta,
}

/// How a discovered file declares its page metadata, in the form the scan could
/// read without a world.
///
/// The two content surfaces name the same thing two ways: markdown writes
/// frontmatter (`+++ title = ".." +++`), BSX writes the component itself
/// (`<Fragment {ArticleMeta{title:".."}}>`). Frontmatter parses to an
/// [`ArticleMeta`] in the scan; a spread is reflect-built at spawn time, where
/// the type registry is in hand.
enum RouteSpecMeta {
	/// A page declaring nothing, ie a `.html` file or markdown with no
	/// frontmatter.
	None,
	/// Markdown frontmatter, parsed during the scan.
	Article(ArticleMeta),
	/// A BSX document's root nodes, whose `ArticleMeta` spread (if any) builds
	/// against the world's type registry.
	Bsx(Vec<BsxNode>),
}

impl RouteSpecMeta {
	/// The page's [`ArticleMeta`], reflect-building a BSX root spread against
	/// the world's type registry (see [`BsxNode::scan_spread`], which reads the
	/// document without building it).
	fn resolve(self, world: &World) -> Option<ArticleMeta> {
		match self {
			Self::None => None,
			Self::Article(meta) => Some(meta),
			Self::Bsx(nodes) => BsxNode::scan_spread(
				&nodes,
				world.get_resource::<AppTypeRegistry>()?,
			),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// Spawn `bundle` and settle the async runtime so the [`RoutesDir`] discovery
	/// task (an async store scan) completes, returning the root entity. Mirrors a
	/// boot path settling before it serves.
	/// Compose `store` on the root (the repo store an entry carries) so the
	/// [`RoutesDir`] resolves it by ancestry, then settle the async runtime so the
	/// discovery task (an async store scan) completes. Mirrors a boot path settling
	/// before it serves.
	async fn spawn_routes(
		world: &mut World,
		store: BlobStore,
		bundle: impl Bundle,
	) -> Entity {
		let root = world.spawn((store, bundle)).flush();
		AsyncRunner::settle_async_tasks(world).await;
		root
	}

	/// Write a routes dir fixture under `target/tests` and return a [`BlobStore`]
	/// backed by an [`FsStore`] rooted at it. Native-only: writes real files.
	#[cfg(not(target_arch = "wasm32"))]
	fn fs_fixture(name: &str, files: &[(&str, &str)]) -> BlobStore {
		let root = fs_ext::workspace_root()
			.join("target/tests/routes_dir")
			.join(name);
		// clean slate so removed fixture files do not leak between runs
		fs_ext::remove(&root).ok();
		for (rel, content) in files {
			fs_ext::write(root.join(rel), content).unwrap();
		}
		BlobStore::new(FsStore::new(AbsPathBuf::new(root).unwrap()))
	}

	/// An in-memory [`BlobStore`] seeded with `files`, proving discovery is
	/// provider-agnostic (the same scan the S3-backed task runs).
	async fn memory_fixture(files: &[(&str, &str)]) -> BlobStore {
		let store = BlobStore::temp();
		for (rel, content) in files {
			store
				.insert(&SmolPath::from(*rel), content.to_string())
				.await
				.unwrap();
		}
		store
	}

	#[beet_core::test]
	fn route_path_of() {
		RoutesDir::route_path_of(&SmolPath::from("docs/intro.md"))
			.xpect_eq(SmolPath::new("docs/intro"));
		RoutesDir::route_path_of(&SmolPath::from("index.md"))
			.xpect_eq(SmolPath::default());
		RoutesDir::route_path_of(&SmolPath::from("docs/index.md"))
			.xpect_eq(SmolPath::new("docs"));
		RoutesDir::route_path_of(&SmolPath::from("about.bsx"))
			.xpect_eq(SmolPath::new("about"));
	}

	/// Assert the three fixture routes render their content, shared by the
	/// filesystem- and memory-backed cases so both providers prove the same scan.
	async fn assert_serves(world: &mut World, root: Entity) {
		for (path, expected) in [
			("", "welcome"),
			("docs/intro", "the intro"),
			("about", "About"),
		] {
			world
				.entity_mut(root)
				.exchange(
					Request::get(path)
						.with_header::<header::Accept>(vec![MediaType::Html]),
				)
				.await
				.unwrap_str()
				.await
				.xpect_contains(expected);
		}
	}

	const SERVES_FILES: &[(&str, &str)] = &[
		("index.md", "# Home\n\nwelcome"),
		("docs/intro.md", "# Intro\n\nthe intro"),
		("about.bsx", "<main><h1>About</h1></main>"),
	];

	/// The filesystem-backed variant: discovery reads real files through an
	/// [`FsStore`]. Native-only (no real fs on wasm); the wasm path is covered by
	/// [`discovers_and_serves_from_memory_store`] over the same files.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	async fn discovers_and_serves_routes() {
		let mut world = router_world();
		let root = spawn_routes(
			&mut world,
			fs_fixture("serves", SERVES_FILES),
			(Router::with_defaults(), children![RoutesDir::default()]),
		)
		.await;
		assert_serves(&mut world, root).await;
	}

	/// The same site loads identically from a non-filesystem store: discovery,
	/// route paths and content reads all go through the [`BlobStore`] abstraction.
	#[beet_core::test]
	async fn discovers_and_serves_from_memory_store() {
		let mut world = router_world();
		let root = spawn_routes(
			&mut world,
			memory_fixture(SERVES_FILES).await,
			(Router::with_defaults(), children![RoutesDir::default()]),
		)
		.await;
		assert_serves(&mut world, root).await;
	}

	/// Discovered files are sorted lexically before spawning, so the [`RouteTree`]
	/// children come out in filename order regardless of store list order. Store
	/// agnostic, so it runs over the in-memory store and covers wasm too.
	#[beet_core::test]
	async fn routes_spawn_in_sorted_order() {
		let mut world = router_world();
		// a bare `Router` (not `Router::with_defaults`) so the opinionated app routes do
		// not appear as extra top-level children alongside the discovered slides.
		// deliberately out-of-order, zero-padded like the slide deck.
		let root = spawn_routes(
			&mut world,
			memory_fixture(&[
				("03-gamma.md", "# Gamma"),
				("01-alpha.md", "# Alpha"),
				("02-beta.md", "# Beta"),
			])
			.await,
			(Router, children![RoutesDir::default()]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		// the discovered slide routes, in tree-child order
		tree.children
			.iter()
			.filter_map(|child| child.path.iter().last())
			.map(|seg| seg.name().to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["01-alpha", "02-beta", "03-gamma"]);
	}

	/// A numbered file declaring a `slug` serves at the slug, not at its
	/// filename, and keeps the filename's number as its nav order — the pair
	/// that lets a directory read in order while its urls stay stable names.
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	async fn slug_overrides_the_filename_path() {
		let mut world = router_world();
		let root = spawn_routes(
			&mut world,
			memory_fixture(&[(
				"blog/1-full-stack-bevy.md",
				"+++\nslug = \"full-stack-bevy\"\n+++\n\n# Post",
			)])
			.await,
			(Router, children![RoutesDir::default()]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		tree.find(&["blog", "1-full-stack-bevy"]).xpect_none();
		let node = tree.find(&["blog", "full-stack-bevy"]).unwrap();
		world
			.entity(node.entity)
			.get::<ArticleMeta>()
			.unwrap()
			.sidebar
			.order
			.unwrap()
			.xpect_eq(1);
	}

	/// A slug renames the page's own segment, and an index file — which has none,
	/// having collapsed into its directory — is told so rather than quietly
	/// renaming the directory out from under its siblings.
	#[beet_core::test]
	fn route_path_for_applies_slug() {
		let slugged = ArticleMeta {
			slug: Some("full-stack-bevy".into()),
			..default()
		};
		RoutesDir::route_path_for(
			&SmolPath::from("blog/1-full-stack-bevy.md"),
			Some(&slugged),
		)
		.unwrap()
		.xpect_eq(SmolPath::new("blog/full-stack-bevy"));
		// no slug declared, the filename stands
		RoutesDir::route_path_for(&SmolPath::from("blog/index.bsx"), None)
			.unwrap()
			.xpect_eq(SmolPath::new("blog"));
		RoutesDir::route_path_for(
			&SmolPath::from("blog/index.bsx"),
			Some(&slugged),
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("no page segment of its own");
	}

	/// A BSX page declares the same metadata markdown puts in frontmatter, as the
	/// component itself on its root; the scan reads it without building the
	/// document, so the route carries it before anyone visits the page.
	#[beet_core::test]
	async fn scan_time_bsx_spread_meta() {
		let mut world = router_world();
		let root = spawn_routes(
			&mut world,
			memory_fixture(&[(
				"blog/index.bsx",
				r#"<Fragment {ArticleMeta{title: "The Full Moon Harvest", created: "2025-09-06", sidebar: SidebarInfo{label: "Blog", order: 1}}}><h1>Blog</h1></Fragment>"#,
			)])
			.await,
			(Router, children![RoutesDir::default()]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		let node = tree.find(&["blog"]).unwrap();
		let meta = world.entity(node.entity).get::<ArticleMeta>().unwrap();
		meta.title
			.as_deref()
			.unwrap()
			.xpect_eq("The Full Moon Harvest");
		meta.sidebar.label.as_deref().unwrap().xpect_eq("Blog");
		meta.sidebar.order.unwrap().xpect_eq(1);
		// the date string coerces to the instant it names
		meta.created
			.unwrap()
			.format_long_date()
			.xpect_eq("6 September 2025");
	}

	/// Frontmatter is scanned from file content through the store, so it is store
	/// agnostic and runs over the in-memory store (covering wasm too).
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	async fn scan_time_frontmatter_meta() {
		let mut world = router_world();
		let root = spawn_routes(
			&mut world,
			memory_fixture(&[(
				"docs/intro.md",
				"+++\ntitle = \"Getting Started\"\norder = 2\n+++\n\n# Intro",
			)])
			.await,
			(Router::with_defaults(), children![RoutesDir::default()]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		let node = tree.find(&["docs", "intro"]).unwrap().clone();
		let meta = world.entity(node.entity).get::<ArticleMeta>().unwrap();
		meta.title.as_deref().unwrap().xpect_eq("Getting Started");
		meta.sidebar.order.unwrap().xpect_eq(2);
	}
}
