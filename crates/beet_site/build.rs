use anyhow::Result;
use beet::prelude::*;
use sweet::prelude::*;

// runtime env vars: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
// cargo:: output https://doc.rust-lang.org/cargo/reference/build-scripts.html#outputs-of-the-build-script
fn main() -> Result<()> {
	println!("cargo::rerun-if-changed=build.rs");
	println!("cargo::rerun-if-changed=../beet_design/src/**/*.mockup.rs");
	println!("cargo::rerun-if-changed=../beet_design/public");
	// println!("cargo::warning={}", "🚀🚀 building beet_site");
	let cx = app_cx!();
	let codegen_wasm = "crates/beet_site/src/codegen/wasm.rs";

	let is_wasm = std::env::var("TARGET").unwrap() == "wasm32-unknown-unknown";

	if is_wasm {
		CodegenFile::new_workspace_rel(codegen_wasm, &cx.pkg_name)
			.with_use_beet_tokens(
				"use beet::prelude::*;use beet::design as beet_design;",
			)
			.xpipe(BuildWasmRoutes::default())?
	} else {
		let html_dir =
			WorkspacePathBuf::new("target/client").into_canonical_unchecked();

		// removing dir breaks the FsWatcher in live reload
		// FsExt::remove(&html_dir).ok();
		let design_public_dir =
			WorkspacePathBuf::new("crates/beet_design/public")
				.into_canonical_unchecked();
		FsExt::copy_recursive(design_public_dir, html_dir)?;


		let pages = FileGroup::new_workspace_rel("crates/beet_site/src/pages")?
			.with_filter(
				GlobFilter::default()
					.with_include("*.rs")
					.with_exclude("*mod.rs"),
			)
			.xpipe(FileGroupToFuncTokens::default())?
			.xpipe(FuncTokensToRsxRoutesGroup::default())
			.xpipe(FuncTokensGroupToCodegen::new(
				CodegenFile::new_workspace_rel(
					"crates/beet_site/src/codegen/pages.rs",
					&cx.pkg_name,
				),
			))?
			.xmap(|(funcs, codegen)| -> Result<_> {
				codegen.build_and_write()?;
				Ok(funcs)
			})?;

		let docs = FileGroup::new_workspace_rel("crates/beet_site/src/docs")?
			.xpipe(FileGroupToFuncTokens::default())?
			.xpipe(MapFuncTokens::default().base_route("/docs"))
			.xpipe(FuncTokensToRsxRoutesGroup::default())
			.xpipe(FuncTokensGroupToCodegen::new(
				CodegenFile::new_workspace_rel(
					"crates/beet_site/src/codegen/docs.rs",
					&cx.pkg_name,
				),
			))?
			.xmap(|(funcs, codegen)| -> Result<_> {
				codegen.build_and_write()?;
				Ok(funcs)
			})?;

		// ⚠️ this is a downstream copy of crates/beet_design/build.rs
		// we're actually only using the route paths, maybe we should generate
		// those in design
		let mockups = FileGroup::new_workspace_rel("crates/beet_design/src")?
			.with_filter(GlobFilter::default().with_include("*.mockup.*"))
			.xpipe(FileGroupToFuncTokens::default())?
			.xpipe(
				MapFuncTokens::default()
					.base_route("/design")
					.replace_route([(".mockup", "")]),
			);

		pages
			.funcs
			.xtend(mockups)
			.xtend(docs.funcs)
			.xpipe(FuncTokensToTree::default())
			.xpipe(FuncTokensTreeToRouteTree {
				codegen_file: CodegenFile::new_workspace_rel(
					"crates/beet_site/src/codegen/route_tree.rs",
					&cx.pkg_name,
				),
			})?;
	}
	Ok(())
}
