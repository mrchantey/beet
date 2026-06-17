//! Build-time route codegen.
//!
//! Scans the site's `src/pages` directory and writes the generated route modules
//! into `src/codegen/`. Run before a `web`/`terminal` build so the generated
//! modules exist:
//!
//! ```not_rust
//! cargo run -p rsx_site --no-default-features --features codegen
//! ```
use beet::prelude::*;

/// Scans the page collection and writes the generated route modules to disk.
pub fn run_codegen() -> Result { async_ext::block_on(route_codegen().export()) }

/// The codegen pass: the typed page collection plus the typed route tree.
fn route_codegen() -> RouteCodegen {
	RouteCodegen::new()
		.add_collection(RouteCollection::new(
			site_rel("src/pages"),
			codegen("pages.rs"),
		))
		.with_route_tree(codegen("route_tree.rs"))
}

/// An absolute path to a file relative to the `rsx_site` crate root.
fn site_rel(path: &str) -> AbsPathBuf {
	AbsPathBuf::new_workspace_rel(format!("examples/rsx_site/{path}")).unwrap()
}

/// A [`CodegenFile`] targeting `src/codegen/<name>`.
fn codegen(name: &str) -> CodegenFile {
	CodegenFile::new(site_rel(&format!("src/codegen/{name}")))
}
