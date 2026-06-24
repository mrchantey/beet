//! Build-time route codegen.
//!
//! Scans the site's `src/pages`, `content` and `actions` collections and writes
//! the generated route modules into `src/codegen/`. Run before a `web`/`terminal`
//! build so the generated modules exist:
//!
//! ```not_rust
//! cargo run -p rsx_site --no-default-features --features codegen
//! ```
use beet::prelude::*;

/// Scans every collection and writes the generated route modules to disk.
pub fn run_codegen() -> Result { async_ext::block_on(route_codegen().export()) }

/// The codegen pass: the typed `pages` collection, the markdown `content`
/// collection, the `actions` server-action collection, plus the typed route tree
/// and the client-action callers.
fn route_codegen() -> RouteCodegen {
	RouteCodegen::new()
		.add_collection(RouteCollection::new(
			site_rel("src/pages"),
			codegen("pages.rs"),
		))
		.add_collection(RouteCollection::new(
			site_rel("content"),
			codegen("content.rs"),
		))
		.add_collection(
			RouteCollection::new(site_rel("actions"), codegen("actions.rs"))
				.with_category(RouteCollectionCategory::Actions)
				.with_server_feature(None::<String>),
		)
		.with_route_tree(codegen("route_tree.rs"))
		.with_client_actions(codegen("client_actions.rs"))
}

/// An absolute path to a file relative to the `rsx_site` crate root.
fn site_rel(path: &str) -> AbsPathBuf {
	AbsPathBuf::new_workspace_rel(format!("examples/rsx_site/{path}")).unwrap()
}

/// A [`CodegenFile`] targeting `src/codegen/<name>`.
fn codegen(name: &str) -> CodegenFile {
	CodegenFile::new(site_rel(&format!("src/codegen/{name}")))
}
