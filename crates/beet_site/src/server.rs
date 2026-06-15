use crate::prelude::*;
use beet::prelude::*;

/// The beet brand green, `rgb(0, 255, 127)` (spring green), seeding the Material
/// palette. Private: every accent derives from it through the style system, so
/// the theme stays the single source of colour.
const THEME_COLOR: Color = Color::srgb(0., 1., 0.75);


/// The site's render substrate, shared by the web and terminal entry points.
///
/// Adds the router observers and charcell pipeline (via [`RouterPlugin`]) plus
/// the Material style rule set that both the web [`Stylesheet`] and the charcell
/// renderer read.
pub fn server_plugin(app: &mut App) {
	app.add_plugins((
		MinimalPlugins,
		// `RouterPlugin` brings in `ServerPlugin`, which installs the `HttpServer`
		// backend and registers the server types.
		RouterPlugin,
		LogPlugin::new(Level::INFO),
		material::MaterialStylePlugin::new(THEME_COLOR),
	));
	// site-local layout rules, see `design_row_rule` (in `style`). The landing-page
	// hero uses `inline_class!` instead, since its layout is a single-use one-off.
	let mut rules = app.world_mut().get_resource_or_init::<RuleSet>();
	rules.insert_rule(design_row_rule());
	// the color-schemes showcase binds each swatch to its role tokens through the
	// rule set, so the palette resolves on both targets rather than a web `<style>`.
	rules.extend_rules(color_scheme_rules());
}

/// Every site route: the page collection plus the docs and blog collections
/// (each grouped under a [`BlobStore`] rooted at its markdown source directory so
/// their [`BlobScene`] routes can read their content), plus the `/assets` route
/// serving the workspace `assets/` directory (eg `/assets/branding/favicon-32x32.png`
/// and the images referenced by blog posts).
pub fn beet_site_endpoints() -> impl Bundle {
	children![
		pages_routes(),
		(content_store("docs"), docs_routes()),
		(content_store("blog"), blog_routes()),
		serve_store("assets", asset_store()),
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

/// The static asset store backing the `/assets` route, rooted at the workspace
/// `assets/` directory.
///
/// A [`BlobStore`] so serving is filesystem/S3 agnostic: [`serve_blob`] streams
/// the bytes from this local [`FsStore`], and swapping in an S3-backed store at
/// deploy time redirects to its public URL instead, with no route change. The
/// directory is git-ignored, holding the branding and blog images the site and
/// markdown reference (eg `/assets/branding/favicon-32x32.png`).
fn asset_store() -> BlobStore {
	BlobStore::new(FsStore::new(
		AbsPathBuf::new_workspace_rel("assets").unwrap(),
	))
}

/// The site router.
///
/// The batteries-included [`default_router`] (adding `/app-info` and
/// `POST /analytics`) wrapped in the global [`BeetLayout`] via the
/// [`BaseLayout`] render middleware, so every route's body is placed into
/// the layout's `<main>`.
pub fn beet_site_router() -> impl Bundle {
	(
		default_router(),
		beet_site_endpoints(),
		BaseLayout::<BeetLayout>::default(),
	)
}
