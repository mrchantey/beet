use anyhow::Result;
use beet::prelude::*;
use sweet::prelude::*;

// runtime env vars: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
// cargo:: output https://doc.rust-lang.org/cargo/reference/build-scripts.html#outputs-of-the-build-script
fn main() -> Result<()> {
	println!("cargo::rerun-if-changed=build.rs");

	// println!("cargo::rerun-if-changed=src/codegen");
	println!("cargo::rerun-if-changed=../beet_design/src/**/*.mockup.rs");
	println!("cargo::rerun-if-changed=../beet_design/public");

	// println!("cargo::warning={}", "🚀🚀 building beet_site");
	let cx = app_cx!();

	let is_wasm = std::env::var("TARGET").unwrap() == "wasm32-unknown-unknown";

	if is_wasm {
		BuildWasmRoutes::new(CodegenFile::new_workspace_rel(
			"crates/beet_site/src/codegen/wasm.rs",
			&cx.pkg_name,
		))
		.run()?;
	} else {
		let html_dir =
			WorkspacePathBuf::new("target/client").into_canonical_unchecked();

		// removing dir makes live reload very hard
		// FsExt::remove(&html_dir).ok();
		let design_public_dir =
			WorkspacePathBuf::new("crates/beet_design/public")
				.into_canonical_unchecked();
		FsExt::copy_recursive(design_public_dir, html_dir)?;


		let mut funcs =
			FileGroup::new_workspace_rel("crates/beet_site/src/routes")?
				.with_filter(
					GlobFilter::default()
						.with_include("*.rs")
						.with_exclude("*mod.rs"),
				)
				.xpipe(FileGroupToFuncTokens::default())?
				.xpipe(FuncTokensToCodegen::new(
					CodegenFile::new_workspace_rel(
						"crates/beet_site/src/codegen/routes.rs",
						&cx.pkg_name,
					),
				))?
				.xmap(|(funcs, codegen)| -> Result<_> {
					codegen.build_and_write()?;
					Ok(funcs)
				})?;

		let docs = FileGroup::new_workspace_rel("crates/beet_site/src/guides")?
			.xpipe(FileGroupToFuncTokens::default())?
			.xpipe(MapFuncTokensRoute::default().base_route("/docs"))
			.xpipe(FuncTokensToCodegen::new(CodegenFile::new_workspace_rel(
				"crates/beet_site/src/codegen/docs.rs",
				&cx.pkg_name,
			)))?
			.xmap(|(funcs, codegen)| -> Result<_> {
				codegen.build_and_write()?;
				Ok(funcs)
			})?;

		// ⚠️ this is a downstream copy of crates/beet_design/build.rs
		let mockups = FileGroup::new_workspace_rel("crates/beet_design/src")?
			.with_filter(GlobFilter::default().with_include("*.mockup.rs"))
			.xpipe(FileGroupToFuncTokens::default())?
			.xpipe(MapFuncTokensRoute::new("/design", [(".mockup", "")]));

		funcs.extend(mockups);
		funcs.extend(docs);

		funcs.xpipe(RouteFuncsToTree {
			codgen_file: CodegenFile::new_workspace_rel(
				"crates/beet_site/src/codegen/route_tree.rs",
				&cx.pkg_name,
			),
		})?;
	}
	Ok(())
}
