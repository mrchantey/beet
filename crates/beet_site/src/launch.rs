//! Build-time route codegen.
//!
//! Scans the site's route directories and writes the generated route modules
//! into `src/codegen/`. Run before a `web`/`terminal` build so the generated
//! modules exist:
//!
//! ```not_rust
//! cargo run -p beet_site --no-default-features --features codegen
//! ```
use beet::prelude::*;

/// Scans every route collection and writes the generated route modules to disk.
pub fn run_codegen() -> Result {
	async_ext::block_on(route_codegen().export())
}

/// The full codegen pass: the page, docs, blog and action collections plus the
/// typed route tree and client-action callers.
fn route_codegen() -> RouteCodegen {
	RouteCodegen::new()
		.add_collection(RouteCollection::new(
			site_rel("src/pages"),
			codegen("pages.rs"),
		))
		.add_collection(
			RouteCollection::new(site_rel("src/docs"), codegen("docs/mod.rs"))
				.with_base_route("docs"),
		)
		.add_collection(
			RouteCollection::new(site_rel("src/blog"), codegen("blog/mod.rs"))
				.with_base_route("blog"),
		)
		.add_collection(
			RouteCollection::new(site_rel("src/actions"), codegen("actions.rs"))
				.with_category(RouteCollectionCategory::Actions)
				.with_server_feature(Some("web")),
		)
		.with_route_tree(codegen("route_tree.rs"))
		.with_client_actions(codegen("client_actions.rs"))
}

/// An absolute path to a file relative to the `beet_site` crate root.
fn site_rel(path: &str) -> AbsPathBuf {
	AbsPathBuf::new_workspace_rel(format!("crates/beet_site/{path}")).unwrap()
}

/// A [`CodegenFile`] targeting `src/codegen/<name>`.
fn codegen(name: &str) -> CodegenFile {
	CodegenFile::new(site_rel(&format!("src/codegen/{name}")))
}
