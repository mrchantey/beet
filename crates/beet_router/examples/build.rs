//! Build script for the test site, this cant be a real build script
//! because:
//! this --> beet_router
//! beet_router/test --> test_site
//! test_site --> this
//!
//! It can still compile because test_site dep is behind a feature flag
use anyhow::Result;
use beet_router::exports::*;
use beet_router::prelude::*;
use sweet::prelude::*;

/// Demonstration of how to collect all files in the 'routes' dir
/// and create a `routes.rs` file containing them all.
pub fn main() -> Result<()> {
	FileGroup::new_workspace_rel("crates/beet_router/src/test_site/pages")?
		.with_filter(
			GlobFilter::default()
				.with_include("*.rs")
				.with_exclude("*mod.rs"),
		)
		.xpipe(FileGroupToFuncTokens::default())?
		.xpipe(FuncTokensToRsxRoutes::new(
			CodegenFile::new_workspace_rel(
				"crates/beet_router/src/test_site/codegen/pages.rs",
				"beet_router",
			)
			.with_import(syn::parse_quote!(
				use crate::as_beet::*;
			)),
		))?
		.xmap(|(_, codegen)| -> Result<_> { codegen.build_and_write() })?;
	println!("success");
	Ok(())
}
