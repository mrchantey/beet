use anyhow::Result;
use beet_router::prelude::*;
use beet_rsx::prelude::*;
use sweet::prelude::ReadFile;
use sweet::prelude::WorkspacePathBuf;

/// Demonstration of how to collect all files in the 'routes' dir
/// and create a `routes.rs` file containing them all.
pub fn main() -> Result<()> {
	let out_file = WorkspacePathBuf::new(
		"crates/beet_router/src/test_site/codegen/routes.rs",
	);
	FileGroup::new_workspace_rel("crates/beet_router/src/test_site/routes")?
		.with_filter(
			GlobFilter::default()
				.with_include("*.rs")
				.with_exclude("*mod.rs"),
		)
		.bpipe(FileGroupToFuncFiles::default())?
		.bpipe(HttpFuncFilesToRouteFuncs::default())?
		.bpipe(RouteFuncsToCodegen::new(
			CodegenFile::new_workspace_rel(
				"crates/beet_router/src/test_site/codegen/routes.rs",
				"beet_router",
			)
			.with_use_beet_tokens("use beet_router::as_beet::*;"),
		))?
		.bmap(|(_, routes, codegen)| -> Result<_> {
			codegen.build_and_write()?;
			Ok(routes)
		})?;

	let file =
		ReadFile::to_string(out_file.into_canonical_unchecked()).unwrap();


	// let routes = parser.build_strings().unwrap();
	println!("wrote {}\n{}", out_file.display(), file);
	Ok(())
}
