//! Build script for the test site,
//! this cant be a real build script because it depends
//! on the crate it builds for, `beet_router`

use anyhow::Result;
use beet_router::prelude::*;
use beet_rsx::prelude::*;

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
		.xpipe(FuncTokensToRsxRoutesGroup::default())
		.xpipe(FuncTokensGroupToCodegen::new(
			CodegenFile::new_workspace_rel(
				"crates/beet_router/src/test_site/codegen/pages.rs",
				"beet_router",
			)
			.with_use_beet_tokens("use beet_router::as_beet::*;"),
		))?
		.xmap(|(_, codegen)| -> Result<_> { codegen.build_and_write() })?;
	Ok(())
}
