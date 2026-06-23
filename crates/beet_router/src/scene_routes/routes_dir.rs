//! Runtime route discovery: a directory of content files becomes routes at
//! spawn time, no codegen.
//!
//! Inserting a [`RoutesDir`] (eg from a `main.bsx` entry via
//! `<RoutesDir src="routes"/>`) triggers [`spawn_routes_dir`]: the nearest ancestor
//! [`BlobStore`] (the site store composed on the loaded root) is scoped to `src` and
//! listed, and each content file
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
/// `docs`). Add a [`PathPartial`] alongside to prefix every discovered route.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RoutesDir {
	/// The content directory, relative to the nearest ancestor [`BlobStore`].
	pub src: String,
}

impl RoutesDir {
	/// Discover routes under `src`, relative to the nearest ancestor [`BlobStore`].
	pub fn new(src: impl Into<String>) -> Self { Self { src: src.into() } }
}

/// The content file extensions served as [`BlobScene`] routes.
const CONTENT_EXTENSIONS: &[&str] = &["md", "mdx", "markdown", "html", "bsx"];

/// Observer: scan the [`RoutesDir`] store and spawn its routes (see the module docs).
///
/// The scan is store I/O (the filesystem in dev, S3 in a deployed task, R2 in a
/// Worker), so it runs as an [`AsyncEntity`] task rather than blocking the runtime
/// (which is single-threaded on wasm). The nearest ancestor [`BlobStore`] (the site
/// store composed on the loaded root) is resolved *inside* that task, where the
/// whole tree is already built, so the ancestor link is reliably present; a
/// store-less app falls back to an [`FsStore`] at the workspace root. The route
/// children therefore appear a few async ticks after the insert, so a boot path
/// settles the async runtime before serving: the Worker entry awaits
/// [`AsyncRunner::settle_async_tasks`](beet_core::prelude::AsyncRunner), the
/// native binary's run loop drives it, and tests await the same settle.
pub fn spawn_routes_dir(
	ev: On<Insert, RoutesDir>,
	dirs: Query<&RoutesDir>,
	mut commands: Commands,
) -> Result {
	let entity = ev.entity;
	let src = SmolPath::from(dirs.get(entity)?.src.as_str());
	// off the async runtime: resolve the nearest ancestor store + scope it to `src`,
	// await the content scan, then compose the scoped store onto the entity, spawn
	// the route children, and flush so the route-tree observers settle against the
	// whole hierarchy.
	commands.entity(entity).queue_async(
		async move |dir: AsyncEntity| -> Result {
			let store = dir
				.with_state::<AncestorQuery<&BlobStore>, BlobStore>(
					|entity, stores| {
						stores
							.get(entity)
							.map(BlobStore::clone)
							.unwrap_or_else(|_| BlobStore::new(FsStore::default()))
					},
				)
				.await?
				.with_subdir(src);
			let specs = discover_routes(&store).await?;
			dir.world()
				.with(move |world| {
					world.entity_mut(entity).insert(store);
					for spec in specs {
						spawn_route_spec(world, entity, spec);
					}
					world.flush();
				})
				.await;
			Ok(())
		},
	);
	Ok(())
}

/// Spawn one discovered content file as a [`BlobScene`] route child of `parent`.
fn spawn_route_spec(world: &mut World, parent: Entity, spec: RouteSpec) {
	#[allow(unused_mut, unused_variables)]
	let mut route_entity = world.spawn((
		ChildOf(parent),
		route(&spec.route_path, BlobScene::new(spec.store_path)),
		HttpMethod::Get,
		ExportStrategy::Static,
		// a discovered content file is a user-facing page, so it carries
		// `PageRoute` and appears in the nav, like its codegen blob equivalent.
		PageRoute,
	));
	// scan-time page metadata, so navigation knows titles/order up front
	#[cfg(feature = "markdown_parser")]
	if let Some(meta) = spec.meta {
		route_entity.insert(meta);
	}
}

/// Wait for every [`RoutesDir`]'s async discovery to finish, for a caller that
/// renders the routes immediately after building (eg the `export-static` /
/// `check` commands).
///
/// [`spawn_routes_dir`] runs the discovery as an async task, so the routes appear
/// a few ticks after the insert. A top-level driver (the Worker, tests) settles
/// the whole runtime with
/// [`AsyncRunner::settle_async_tasks`](beet_core::prelude::AsyncRunner). A caller
/// running *inside* the app (an action) cannot drive the loop without re-entering
/// it, so this yields (via the world bridge) to let the runtime drive the task,
/// detecting completion by the scoped store the task composes onto each
/// [`RoutesDir`] entity. Capped so an unresolvable scan errors rather than hangs.
pub async fn settle_routes_dirs(world: &AsyncWorld) -> Result {
	for _ in 0..10_000 {
		let pending = world
			.with(|world| {
				// routes still discovering (no scoped store composed onto them yet) ...
				let dirs = world
					.query_filtered::<(), (With<RoutesDir>, Without<BlobStore>)>()
					.iter(world)
					.count();
				// ... plus any unresolved `<Template src>` include: it builds the
				// included entry (and its own `RoutesDir`) asynchronously as a pending
				// dependency, so wait for the set to drain before reading the tree.
				let includes = world
					.query::<&TemplatePending>()
					.iter(world)
					.filter(|pending| !pending.is_empty())
					.count();
				dirs + includes
			})
			.await;
		if pending == 0 {
			return Ok(());
		}
		// yield so the runtime can drive the discovery/include tasks between checks.
		async_ext::yield_now().await;
	}
	bevybail!(
		"RoutesDir discovery / `<Template src>` includes did not settle within the \
		frame budget"
	)
}

/// A discovered content file: its route path, the store path its bytes load from,
/// and any scan-time frontmatter metadata.
struct RouteSpec {
	route_path: String,
	store_path: SmolPath,
	#[cfg(feature = "markdown_parser")]
	meta: Option<ArticleMeta>,
}

/// List the store's content files and read each markdown file's frontmatter,
/// returning route specs in lexical path order so zero-padded routes (eg slides
/// `01..20`) spawn in sequence, giving a deterministic [`RouteTree`] child order.
async fn discover_routes(store: &BlobStore) -> Result<Vec<RouteSpec>> {
	let mut paths = store.list().await?;
	paths.sort();
	paths
		.into_iter()
		.filter(is_content)
		.map(async |path| -> Result<RouteSpec> {
			Ok(RouteSpec {
				route_path: route_path_of(&path),
				#[cfg(feature = "markdown_parser")]
				meta: scan_meta(store, &path).await,
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

/// The route path of a content file: the extension is dropped and a trailing
/// `index` collapses to its directory, eg `docs/index.md` -> `docs`.
fn route_path_of(rel: &SmolPath) -> String {
	let mut segments = rel.segments();
	if let (Some(stem), Some(last)) = (rel.file_stem(), segments.last_mut()) {
		*last = stem;
	}
	if segments.last() == Some(&"index") {
		segments.pop();
	}
	segments.join("/")
}

/// Read a markdown file's leading frontmatter into [`ArticleMeta`] through the
/// store, if it is markdown and parses; any read/parse failure yields `None`.
#[cfg(feature = "markdown_parser")]
async fn scan_meta(store: &BlobStore, path: &SmolPath) -> Option<ArticleMeta> {
	let is_markdown = path
		.extension()
		.is_some_and(|ext| matches!(ext, "md" | "mdx" | "markdown"));
	if !is_markdown {
		return None;
	}
	let bytes = store.get(path).await.ok()?;
	ArticleMeta::from_markdown(core::str::from_utf8(&bytes).ok()?)
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
	/// Compose `store` on the root (the site store an entry carries) so the
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
		use super::route_path_of;
		route_path_of(&SmolPath::from("docs/intro.md")).xpect_eq("docs/intro");
		route_path_of(&SmolPath::from("index.md")).xpect_eq("");
		route_path_of(&SmolPath::from("docs/index.md")).xpect_eq("docs");
		route_path_of(&SmolPath::from("about.bsx")).xpect_eq("about");
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
			(default_router(), children![RoutesDir::new("")]),
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
			(default_router(), children![RoutesDir::new("")]),
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
		// a bare `Router` (not `default_router`) so the opinionated app routes do
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
			(Router, children![RoutesDir::new("")]),
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
			(default_router(), children![RoutesDir::new("")]),
		)
		.await;

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		let node = tree.find(&["docs", "intro"]).unwrap().clone();
		let meta = world.entity(node.entity).get::<ArticleMeta>().unwrap();
		meta.title.as_deref().unwrap().xpect_eq("Getting Started");
		meta.sidebar.order.unwrap().xpect_eq(2);
	}
}
