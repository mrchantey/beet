use crate::prelude::*;
use beet::prelude::*;

/// The site's render substrate, shared by the web and terminal entry points.
///
/// Adds the router observers and charcell pipeline (via [`RouterPlugin`]) plus
/// the Material style rule set that both the web [`Stylesheet`] and the charcell
/// renderer read.
pub fn server_plugin(app: &mut App) {
	app.add_plugins((
		MinimalPlugins,
		RouterPlugin,
		ServerPlugin,
		material::MaterialStylePlugin::new(palettes::basic::GREEN),
	));
}

/// Every site route: the page collection plus the docs and blog collections,
/// each grouped under a [`BlobStore`] rooted at its markdown source directory so
/// their [`BlobScene`] routes can read their content.
pub fn beet_site_endpoints() -> impl Bundle {
	children![
		pages_routes(),
		(content_store("docs"), docs_routes()),
		(content_store("blog"), blog_routes()),
	]
}

/// A [`BlobStore`] rooted at `crates/beet_site/src/<dir>`, the source directory
/// of a markdown collection.
fn content_store(dir: &str) -> BlobStore {
	let root =
		AbsPathBuf::new_workspace_rel(format!("crates/beet_site/src/{dir}"))
			.unwrap();
	BlobStore::new(FsStore::new(root))
}

/// The site router.
///
/// The batteries-included [`default_router`] (adding `/app-info` and
/// `POST /analytics`) wrapped in the global [`BeetDocumentShell`] via the
/// [`DocumentShell`] render middleware, so every route's body is placed into
/// the shell's `<main>`.
pub fn beet_site_router() -> impl Bundle {
	(
		default_router(),
		beet_site_endpoints(),
		DocumentShell::<BeetDocumentShell>::default(),
	)
}
