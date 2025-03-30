use anyhow::Result;
use beet_router::as_beet::*;

fn main() -> Result<()> {
	let cx = app_cx!();
	println!("cargo::rerun-if-changed=build.rs");
	// mockups can be generated from anywhere in src,
	// so rebuild if any change
	println!("cargo::rerun-if-changed=src");

	FileGroup::new_workspace_rel("crates/beet_design/src")?
		.with_filter(GlobFilter::default().with_include("*.mockup.rs"))
		.bpipe(FileGroupToFuncFiles::default())?
		.bpipe(FuncFilesToRouteFuncs::mockups())?
		.bpipe(RouteFuncsToCodegen::new(
			CodegenFile::new_workspace_rel(
				"crates/beet_design/src/codegen/mockups.rs",
				&cx.pkg_name,
			)
			.with_use_beet_tokens("use beet_router::as_beet::*;"),
		))?
		.bmap(|(_, _, codegen)| codegen)
		.build_and_write()?;

	Ok(())
}
