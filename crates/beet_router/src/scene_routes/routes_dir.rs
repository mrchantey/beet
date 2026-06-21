//! Runtime route discovery: a directory of content files becomes routes at
//! spawn time, no codegen.
//!
//! Inserting a [`RoutesDir`] (eg from a `main.bsx` entry via
//! `<RoutesDir src="routes"/>`) triggers [`spawn_routes_dir`]: the [`SiteRoot`]
//! store is scoped to `src` and listed, and each content file
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
	/// The content directory, relative to [`SiteRoot`].
	pub src: String,
}

impl RoutesDir {
	/// Discover routes under `src`, relative to [`SiteRoot`].
	pub fn new(src: impl Into<String>) -> Self { Self { src: src.into() } }
}

/// The content file extensions served as [`BlobScene`] routes.
const CONTENT_EXTENSIONS: &[&str] = &["md", "mdx", "markdown", "html", "bsx"];

/// Observer: scan the [`RoutesDir`] store and spawn its routes (see the module docs).
///
/// Native-only: it blocks on the store scan at spawn time. The wasm site-load
/// path (the Cloudflare Worker) cannot block, so it awaits
/// [`spawn_routes_dir_async`] after the entry build instead.
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_routes_dir(
	ev: On<Insert, RoutesDir>,
	dirs: Query<&RoutesDir>,
	site_root: Option<Res<SiteRoot>>,
	mut commands: Commands,
) -> Result {
	// the site store scoped to `src`; the BlobScene routes read their bytes from it.
	let store = site_root
		.map(|root| root.0.clone())
		.unwrap_or_else(|| SiteRoot::default().0)
		.with_subdir(SmolPath::from(dirs.get(ev.entity)?.src.as_str()));
	commands.entity(ev.entity).insert(store.clone());

	// discover routes + read frontmatter through the store, a one-time blocking
	// scan at spawn (boot): fs-backed in dev, S3-backed in a deployed task.
	let specs = async_ext::block_on(discover_routes(&store))?;
	for spec in specs {
		#[allow(unused_mut, unused_variables)]
		let mut route_entity = commands.spawn((
			ChildOf(ev.entity),
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
	Ok(())
}

/// Async counterpart to [`spawn_routes_dir`] for wasm, where a blocking scan
/// would hang the single-threaded runtime.
///
/// Resolves the [`SiteRoot`]-scoped store for every [`RoutesDir`] in the world,
/// awaits its content scan, then spawns each route child and flushes so the
/// route-tree observers settle. Run once after the entry build, since the
/// build's `Insert, RoutesDir` observer is native-only.
#[cfg(target_arch = "wasm32")]
pub async fn spawn_routes_dir_async(world: &mut World) -> Result {
	// snapshot the (entity, scoped store) pairs, the SiteRoot scoped to each `src`.
	let site_store = world
		.get_resource::<SiteRoot>()
		.map(|root| root.0.clone())
		.unwrap_or_else(|| SiteRoot::default().0);
	let dirs = world
		.query::<(Entity, &RoutesDir)>()
		.iter(world)
		.map(|(entity, dir)| {
			(entity, site_store.with_subdir(SmolPath::from(dir.src.as_str())))
		})
		.collect::<Vec<_>>();

	for (entity, store) in dirs {
		// compose the scoped store onto the entity so its routes read from it.
		world.entity_mut(entity).insert(store.clone());
		// await the content scan (the wasm-safe replacement for the native
		// observer's `block_on`), then spawn the route children directly.
		for spec in discover_routes(&store).await? {
			spawn_route_spec(world, entity, spec);
		}
		// flush so the `Insert, PathPattern` route-tree observers run against the
		// settled hierarchy before the next exchange.
		world.flush();
	}
	Ok(())
}

/// Spawn one discovered content file as a [`BlobScene`] route child of `parent`.
#[cfg(target_arch = "wasm32")]
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

	/// Write a routes dir fixture under `target/tests` and return a [`SiteRoot`]
	/// backed by an [`FsStore`] rooted at it.
	fn fs_fixture(name: &str, files: &[(&str, &str)]) -> SiteRoot {
		let root = fs_ext::workspace_root()
			.join("target/tests/routes_dir")
			.join(name);
		// clean slate so removed fixture files do not leak between runs
		fs_ext::remove(&root).ok();
		for (rel, content) in files {
			fs_ext::write(root.join(rel), content).unwrap();
		}
		SiteRoot::new_fs(AbsPathBuf::new(root).unwrap())
	}

	/// A [`SiteRoot`] backed by an in-memory store seeded with `files`, proving
	/// discovery is provider-agnostic (the same scan the S3-backed task runs).
	async fn memory_fixture(files: &[(&str, &str)]) -> SiteRoot {
		let store = BlobStore::temp();
		for (rel, content) in files {
			store
				.insert(&SmolPath::from(*rel), content.to_string())
				.await
				.unwrap();
		}
		SiteRoot(store)
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

	#[beet_core::test]
	async fn discovers_and_serves_routes() {
		let mut world = router_world();
		world.insert_resource(fs_fixture("serves", SERVES_FILES));
		let root = world
			.spawn((default_router(), children![RoutesDir::new("")]))
			.flush();
		assert_serves(&mut world, root).await;
	}

	/// The same site loads identically from a non-filesystem store: discovery,
	/// route paths and content reads all go through the [`BlobStore`] abstraction.
	#[beet_core::test]
	async fn discovers_and_serves_from_memory_store() {
		let mut world = router_world();
		world.insert_resource(memory_fixture(SERVES_FILES).await);
		let root = world
			.spawn((default_router(), children![RoutesDir::new("")]))
			.flush();
		assert_serves(&mut world, root).await;
	}

	/// Discovered files are sorted lexically before spawning, so the [`RouteTree`]
	/// children come out in filename order regardless of store list order.
	#[beet_core::test]
	fn routes_spawn_in_sorted_order() {
		let mut world = router_world();
		// deliberately out-of-order, zero-padded like the slide deck
		world.insert_resource(fs_fixture("sorted", &[
			("03-gamma.md", "# Gamma"),
			("01-alpha.md", "# Alpha"),
			("02-beta.md", "# Beta"),
		]));
		// a bare `Router` (not `default_router`) so the opinionated app routes do
		// not appear as extra top-level children alongside the discovered slides.
		let root = world.spawn((Router, children![RoutesDir::new("")])).flush();

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		// the discovered slide routes, in tree-child order
		tree.children
			.iter()
			.filter_map(|child| child.path.iter().last())
			.map(|seg| seg.name().to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["01-alpha", "02-beta", "03-gamma"]);
	}

	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	fn scan_time_frontmatter_meta() {
		let mut world = router_world();
		world.insert_resource(fs_fixture("meta", &[(
			"docs/intro.md",
			"+++\ntitle = \"Getting Started\"\norder = 2\n+++\n\n# Intro",
		)]));
		let root = world
			.spawn((default_router(), children![RoutesDir::new("")]))
			.flush();

		let tree = world.entity(root).get::<RouteTree>().unwrap().clone();
		let node = tree.find(&["docs", "intro"]).unwrap().clone();
		let meta = world.entity(node.entity).get::<ArticleMeta>().unwrap();
		meta.title.as_deref().unwrap().xpect_eq("Getting Started");
		meta.sidebar.order.unwrap().xpect_eq(2);
	}
}
