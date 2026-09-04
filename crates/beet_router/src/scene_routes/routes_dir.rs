//! Runtime route discovery: a directory of content files becomes routes at
//! spawn time, no codegen.
//!
//! Inserting a [`RoutesDir`] (eg from a `main.bsx` entry via
//! `<RoutesDir src="routes"/>`) triggers [`RoutesDir::spawn_on_insert`]: the
//! nearest ancestor [`BlobStore`] (the repo store composed on the loaded root) is
//! scoped to `src` and listed, and each content file
//! (`.md`/`.mdx`/`.bsx`/`.html`) spawns a [`BlobScene`] route child served through
//! the shared media-parse pipeline. The scoped [`BlobStore`] is composed onto the
//! [`RoutesDir`] entity so the routes read their bytes from it, and each file's
//! ROOT declarations ([`RootDeclarations`]: markdown frontmatter or a BSX root
//! spread) are read at scan time and hoisted onto the route entity, so navigation
//! (eg [`RouteSidebar`](crate::prelude::RouteSidebar)) knows every page's
//! title/order without visiting it. The scan knows no metadata type: it hoists
//! whatever components a document declares, and [`PageMeta`] is one consumer of
//! that set like any other. Discovery is store-backed, so it reads identically
//! from the local filesystem in dev and from S3 in a deployed task.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

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
///
/// A [`filter`](Self::filter) narrows which files are discovered, so one
/// directory can be served by several dirs: the root excludes a subtree another
/// scans under its own [`Route`](crate::prelude::route), giving that subtree its
/// own layout and redirects while the urls stay where they were.
///
/// ```bsx
/// <RoutesDir src="routes" filter={GlobFilter{exclude:["blog/**"]}}/>
/// <Route path="blog" {Layout{template:"ArticleLayout"}}>
///     <RoutesDir src="routes/blog"/>
/// </Route>
/// ```
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RoutesDir {
	/// The content directory, relative to the nearest ancestor [`BlobStore`].
	pub src: String,
	/// Which of `src`'s content files to serve, matched against each file's
	/// path relative to `src` (eg `blog/1-post.md`). Open by default.
	///
	/// A bare string or list authors the include allowlist
	/// (`filter="docs/**"`); the struct literal names either list
	/// (`filter={GlobFilter{exclude:["blog/**"]}}`).
	pub filter: GlobFilter,
}

/// The content file extensions served as [`BlobScene`] routes.
const CONTENT_EXTENSIONS: &[&str] = &["md", "mdx", "markdown", "html", "bsx"];

impl RoutesDir {
	/// Discover routes under `src`, relative to the nearest ancestor [`BlobStore`].
	pub fn new(src: impl Into<String>) -> Self {
		Self {
			src: src.into(),
			..default()
		}
	}

	/// Discover only the files passing `filter`, matched against each path
	/// relative to `src`.
	pub fn with_filter(mut self, filter: GlobFilter) -> Self {
		self.filter = filter;
		self
	}

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
		let dir = dirs.get(entity)?;
		let src = SmolPath::from(dir.src.as_str());
		let filter = dir.filter.clone();
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
					// resolved together, inside the task where the whole tree is
					// built so the ancestor links are reliably present: the store
					// the files load from, and the component this dir's unsectioned
					// frontmatter keys declare.
					let (store, frontmatter_type) = dir
						.with_state::<(
							AncestorQuery<&BlobStore>,
							AncestorQuery<&FrontmatterType>,
						), Result<(BlobStore, FrontmatterType)>>(
							|entity, (stores, types)| {
								Ok((
									stores.get(entity).cloned()?,
									types
										.get(entity)
										.cloned()
										.unwrap_or_default(),
								))
							},
						)
						.await??;
					let store = store.with_subdir(src);
					let specs = Self::discover_routes(
						&store,
						&filter,
						&frontmatter_type.component,
					)
					.await?;
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

	/// Spawn one discovered content file as a [`BlobScene`] route child of `parent`,
	/// hoisting the components its root declared onto the route entity.
	///
	/// The declarations resolve here rather than in the scan because
	/// reflect-building them needs the world's type registry, and the router reads
	/// [`PageMeta`] out of them first because a `slug` has the last word on the
	/// url — before the route entity it would live on exists.
	fn spawn_route_spec(
		world: &mut World,
		parent: Entity,
		mut spec: RouteSpec,
	) -> Result {
		PageMeta::declare_file_defaults(
			&mut spec.declarations,
			&spec.store_path,
		);
		let meta = world
			.get_resource::<AppTypeRegistry>()
			.and_then(|registry| spec.declarations.get(&registry.read()));
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
		spec.declarations.insert(&mut route_entity)
	}

	/// List the store's content files and read each one's declared metadata,
	/// returning route specs in lexical path order so zero-padded routes (eg slides
	/// `01..20`) spawn in sequence, giving a deterministic [`RouteTree`] child order.
	///
	/// This half is the store I/O; what the bytes MEAN settles at spawn time (see
	/// [`spawn_route_spec`](Self::spawn_route_spec)), which is also where a `slug`
	/// renames the route path — after the sort, so it cannot reshuffle the order.
	async fn discover_routes(
		store: &BlobStore,
		filter: &GlobFilter,
		frontmatter_type: &str,
	) -> Result<Vec<RouteSpec>> {
		let mut paths = store.list().await?;
		paths.sort();
		paths
			.into_iter()
			.filter(|path| Self::is_content(path) && filter.passes(path))
			.map(async |path| -> Result<RouteSpec> {
				Ok(RouteSpec {
					declarations: Self::scan_declarations(
						store,
						&path,
						frontmatter_type,
					)
					.await,
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
	/// the resolved [`PageMeta`] (see [`PageMeta::apply_slug`]).
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
		meta: Option<&PageMeta>,
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

	/// Read a content file's ROOT declarations through the store: markdown
	/// frontmatter, or the root spreads of a BSX document. Any read/parse failure
	/// yields no declarations, since a page declaring nothing is a page.
	async fn scan_declarations(
		store: &BlobStore,
		path: &SmolPath,
		frontmatter_type: &str,
	) -> RootDeclarations {
		let Some(source) = store
			.get(path)
			.await
			.ok()
			.and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
		else {
			return default();
		};
		match path.extension() {
			Some("md" | "mdx" | "markdown") => Frontmatter::extract(&source)
				.ok()
				.flatten()
				.map(|frontmatter| frontmatter.declarations(frontmatter_type))
				.unwrap_or_default(),
			Some("bsx") => BsxNode::parse_document(&source, &default())
				.map(|nodes| RootDeclarations::from_bsx(&nodes))
				.unwrap_or_default(),
			_ => default(),
		}
	}
}

/// A discovered content file: the store path its bytes load from, and the
/// components that file declares at its root.
struct RouteSpec {
	store_path: SmolPath,
	declarations: RootDeclarations,
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

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
			.get::<PageMeta>()
			.unwrap()
			.order
			.unwrap()
			.xpect_eq(1);
	}

	/// A slug renames the page's own segment, and an index file — which has none,
	/// having collapsed into its directory — is told so rather than quietly
	/// renaming the directory out from under its siblings.
	#[beet_core::test]
	fn route_path_for_applies_slug() {
		let slugged = PageMeta {
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

	/// A filtered root dir and a scoped dir under a `<Route>` compose to the urls
	/// one unfiltered dir would serve: the excluded subtree is discovered by the
	/// second dir instead, which is what lets it carry its own layout.
	#[beet_core::test]
	async fn filter_splits_a_dir_across_two_scans() {
		let mut world = router_world();
		let files = &[
			("index.md", "# Home"),
			("docs/intro.md", "# Intro"),
			("blog/index.md", "# Blog"),
			("blog/1-post.md", "# Post"),
		];
		let root = spawn_routes(
			&mut world,
			memory_fixture(files).await,
			(Router, children![
				RoutesDir::default()
					.with_filter(GlobFilter::default().with_exclude("blog/**")),
				(PathPartial::new("blog"), children![RoutesDir::new("blog")])
			]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		// the blog subtree serves at the same urls the single-dir scan gave it
		tree.find(&["blog"]).xpect_some();
		tree.find(&["blog", "1-post"]).xpect_some();
		tree.find(&["docs", "intro"]).xpect_some();
		// ..and the root dir discovered it once, not twice (a duplicate route
		// would have failed the tree build outright)
		tree.find(&["blog", "blog"]).xpect_none();
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
				r#"<Fragment {PageMeta{title: "The Full Moon Harvest", created: "2025-09-06", sidebar_label: "Blog", order: 1}}><h1>Blog</h1></Fragment>"#,
			)])
			.await,
			(Router, children![RoutesDir::default()]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		let node = tree.find(&["blog"]).unwrap();
		let meta = world.entity(node.entity).get::<PageMeta>().unwrap();
		meta.title
			.as_deref()
			.unwrap()
			.xpect_eq("The Full Moon Harvest");
		meta.sidebar_label.as_deref().unwrap().xpect_eq("Blog");
		meta.order.unwrap().xpect_eq(1);
		// the date string coerces to the instant it names
		meta.created
			.unwrap()
			.format_long_date()
			.xpect_eq("6 September 2025");
	}

	/// The scan hoists WHATEVER a document declares, not one blessed type: a
	/// TOML `[Section]` names its component by short type path and lands beside
	/// the default component's keys.
	#[beet_core::test]
	async fn hoists_sectioned_components() {
		let mut world = router_world();
		let root = spawn_routes(
			&mut world,
			memory_fixture(&[(
				"blog/post.md",
				"+++\ntitle = \"Post\"\n[Layout]\ntemplate = \"ArticleLayout\"\n+++\n\n# Post",
			)])
			.await,
			(Router, children![RoutesDir::default()]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		let entity = tree.find(&["blog", "post"]).unwrap().entity;
		world
			.entity(entity)
			.get::<PageMeta>()
			.unwrap()
			.title
			.as_deref()
			.unwrap()
			.xpect_eq("Post");
		world
			.entity(entity)
			.get::<Layout>()
			.unwrap()
			.template
			.as_str()
			.xpect_eq("ArticleLayout");
	}

	/// A dir declaring its own [`FrontmatterType`] redirects the unsectioned
	/// keys, so a site's own metadata component needs no change to the scan.
	#[beet_core::test]
	async fn frontmatter_type_overrides_the_default_component() {
		let mut world = router_world();
		let root = spawn_routes(
			&mut world,
			memory_fixture(&[(
				"post.md",
				"+++\ntemplate = \"ArticleLayout\"\n+++\n\n# Post",
			)])
			.await,
			(Router, children![(RoutesDir::default(), FrontmatterType {
				component: "Layout".into()
			})]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		let entity = tree.find(&["post"]).unwrap().entity;
		world
			.entity(entity)
			.get::<Layout>()
			.unwrap()
			.template
			.as_str()
			.xpect_eq("ArticleLayout");
		world
			.entity(entity)
			.get::<PageMeta>()
			.is_none()
			.xpect_true();
	}

	/// A section naming a component this binary does not register warns and is
	/// skipped: the page still serves, carrying every declaration that did
	/// resolve, exactly as an unregistered tag still builds its subtree.
	#[beet_core::test]
	async fn unknown_section_does_not_take_the_page_down() {
		let mut world = router_world();
		let root = spawn_routes(
			&mut world,
			memory_fixture(&[(
				"post.md",
				"+++\ntitle = \"Post\"\n[NotInThisBinary]\nfoo = \"bar\"\n+++\n\n# Post",
			)])
			.await,
			(Router, children![RoutesDir::default()]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		let entity = tree.find(&["post"]).unwrap().entity;
		world
			.entity(entity)
			.get::<PageMeta>()
			.unwrap()
			.title
			.as_deref()
			.unwrap()
			.xpect_eq("Post");
	}

	/// Frontmatter is scanned from file content through the store, so it is store
	/// agnostic and runs over the in-memory store (covering wasm too).
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
		let meta = world.entity(node.entity).get::<PageMeta>().unwrap();
		meta.title.as_deref().unwrap().xpect_eq("Getting Started");
		meta.order.unwrap().xpect_eq(2);
	}
}
