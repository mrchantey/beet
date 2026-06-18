//! Runtime route discovery: a directory of content files becomes routes at
//! spawn time, no codegen.
//!
//! Inserting a [`RoutesDir`] (eg from a `main.bsx` entry via
//! `<RoutesDir src="routes"/>`) triggers [`spawn_routes_dir`]: the directory is
//! scanned recursively and each content file (`.md`/`.mdx`/`.bsx`/`.html`) spawns a
//! [`BlobScene`] route child, served through the shared media-parse pipeline.
//! A [`BlobStore`] rooted at the directory backs the routes, and markdown
//! frontmatter is read at scan time into [`ArticleMeta`] so navigation (eg
//! [`RouteSidebar`](crate::prelude::RouteSidebar)) knows every page's
//! title/order without visiting it.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use std::path::Path;

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

/// Observer: scan the [`RoutesDir`] and spawn its routes (see the module docs).
pub fn spawn_routes_dir(
	ev: On<Insert, RoutesDir>,
	dirs: Query<&RoutesDir>,
	site_root: Option<Res<SiteRoot>>,
	mut commands: Commands,
) -> Result {
	let dir = site_root
		.map(|root| root.0.clone())
		.unwrap_or_else(|| SiteRoot::default().0)
		.join(&dirs.get(ev.entity)?.src);

	// the store the BlobScene routes read their bytes from
	commands
		.entity(ev.entity)
		.insert(BlobStore::new(FsStore::new(dir.clone())));

	// `files_recursive` yields filesystem order (unspecified); sort lexically by
	// relative path so zero-padded routes (eg slides `01..20`) spawn in sequence,
	// giving a deterministic `RouteTree` child order for sibling navigation.
	let mut files = ReadDir::files_recursive(&dir)?;
	files.sort();
	for file in files {
		let is_content = file
			.extension()
			.and_then(|ext| ext.to_str())
			.is_some_and(|ext| CONTENT_EXTENSIONS.contains(&ext));
		if !is_content {
			continue;
		}
		let rel = file.strip_prefix(&dir).map_err(|_| {
			bevyhow!(
				"file `{}` is not under `{}`",
				file.display(),
				dir.display()
			)
		})?;
		#[allow(unused_mut, unused_variables)]
		let mut route_entity = commands.spawn((
			ChildOf(ev.entity),
			route(&route_path_of(rel), BlobScene::new(store_path_of(rel))),
			HttpMethod::Get,
			ExportStrategy::Static,
			// a discovered content file is a user-facing page, so it carries
			// `PageRoute` and appears in the nav, like its codegen blob equivalent.
			PageRoute,
		));
		// scan-time page metadata, so navigation knows titles/order up front
		#[cfg(feature = "markdown_parser")]
		if let Some(meta) = scan_meta(&file) {
			route_entity.insert(meta);
		}
	}
	Ok(())
}

/// A relative path's `/`-joined utf8 segments, cross-platform.
fn segments_of(rel: &Path) -> Vec<&str> {
	rel.components()
		.filter_map(|component| component.as_os_str().to_str())
		.collect()
}

/// The route path of a content file: the extension is dropped and a trailing
/// `index` collapses to its directory, eg `docs/index.md` -> `docs`.
fn route_path_of(rel: &Path) -> String {
	let without_ext = rel.with_extension("");
	let mut segments = segments_of(&without_ext);
	if segments.last() == Some(&"index") {
		segments.pop();
	}
	segments.join("/")
}

/// A content file's store path: its `/`-joined segments, extension kept.
fn store_path_of(rel: &Path) -> String { segments_of(rel).join("/") }

/// Parse a markdown file's leading frontmatter into [`ArticleMeta`], if any.
#[cfg(feature = "markdown_parser")]
fn scan_meta(file: &Path) -> Option<ArticleMeta> {
	let is_markdown = file
		.extension()
		.and_then(|ext| ext.to_str())
		.is_some_and(|ext| matches!(ext, "md" | "mdx" | "markdown"));
	if !is_markdown {
		return None;
	}
	fs_ext::read_to_string(file)
		.ok()
		.and_then(|source| ArticleMeta::from_markdown(&source))
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// Write a routes dir fixture under `target/tests` and return a `SiteRoot`
	/// pointing at it.
	fn fixture(name: &str, files: &[(&str, &str)]) -> SiteRoot {
		let root = fs_ext::workspace_root()
			.join("target/tests/routes_dir")
			.join(name);
		// clean slate so removed fixture files do not leak between runs
		fs_ext::remove(&root).ok();
		for (rel, content) in files {
			fs_ext::write(root.join(rel), content).unwrap();
		}
		SiteRoot(AbsPathBuf::new(root).unwrap())
	}

	#[beet_core::test]
	fn route_path_of() {
		use super::route_path_of;
		use std::path::Path;
		route_path_of(Path::new("docs/intro.md")).xpect_eq("docs/intro");
		route_path_of(Path::new("index.md")).xpect_eq("");
		route_path_of(Path::new("docs/index.md")).xpect_eq("docs");
		route_path_of(Path::new("about.bsx")).xpect_eq("about");
	}

	#[beet_core::test]
	async fn discovers_and_serves_routes() {
		let mut world = router_world();
		world.insert_resource(fixture("serves", &[
			("index.md", "# Home\n\nwelcome"),
			("docs/intro.md", "# Intro\n\nthe intro"),
			("about.bsx", "<main><h1>About</h1></main>"),
		]));
		let root = world
			.spawn((default_router(), children![RoutesDir::new("")]))
			.flush();

		for (path, expected) in [
			("", "welcome"),
			("docs/intro", "the intro"),
			("about", "About"),
		] {
			world
				.entity_mut(root)
				.call::<Request, Response>(
					Request::get(path)
						.with_header::<header::Accept>(vec![MediaType::Html]),
				)
				.await
				.unwrap()
				.unwrap_str()
				.await
				.xpect_contains(expected);
		}
	}

	/// Discovered files are sorted lexically before spawning, so the [`RouteTree`]
	/// children come out in filename order regardless of filesystem read order.
	#[beet_core::test]
	fn routes_spawn_in_sorted_order() {
		let mut world = router_world();
		// deliberately out-of-order, zero-padded like the slide deck
		world.insert_resource(fixture("sorted", &[
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
		world.insert_resource(fixture("meta", &[(
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
