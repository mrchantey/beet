use anyhow::Result;
use beet_router::prelude::*;
use beet_rsx::prelude::*;
use sweet::prelude::WorkspacePathBuf;

/// Demonstration of how to collect all files in the 'routes' dir
/// and create a `routes.rs` file containing them all.
pub fn main() -> Result<()> {
	WorkspacePathBuf::new("crates/beet_router/src/test_site/codegen/routes.rs");
	FileGroup::new_workspace_rel("crates/beet_router/src/test_site/routes")?
		.with_filter(
			GlobFilter::default()
				.with_include("*.rs")
				.with_exclude("*mod.rs"),
		)
		.xpipe(FileGroupToFuncTokens::default())?
		.xpipe(FuncTokensToCodegen::new(
			CodegenFile::new_workspace_rel(
				"crates/beet_router/src/test_site/codegen/routes.rs",
				"beet_router",
			)
			.with_use_beet_tokens("use beet_router::as_beet::*;"),
		))?
		.xmap(|(_, codegen)| -> Result<_> { codegen.build_and_write() })?;
	Ok(())
}
