use anyhow::Result;
use beet::exports::*;
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

	let is_wasm = std::env::var("TARGET").unwrap() == "wasm32-unknown-unknown";

	if is_wasm {
		CodegenFile::new(AbsPathBuf::new_manifest_rel_unchecked(
			"src/codegen/wasm.rs",
		))
		.with_pkg_name(&cx.pkg_name)
		.with_import(syn::parse_quote!(
			use beet::design as beet_design;
		))
		.xpipe(BuildWasmRoutes::default())?
	} else {
		// removing dir breaks the FsWatcher in live reload
		// FsExt::remove(&html_dir).ok();


		let _client_actions =
			FileGroup::new(AbsPathBuf::new_manifest_rel("src/actions")?)
				.xpipe(FileGroupToFuncTokens::default())?
				.xmap(|g| g.into_tree())
				.xpipe(FuncTokensTreeToServerActions::new(
					CodegenFile::new(AbsPathBuf::new_manifest_rel_unchecked(
						"src/codegen/client_actions.rs",
					))
					.with_pkg_name(&cx.pkg_name),
				))?;
		let _server_actions =
			FileGroup::new(AbsPathBuf::new_manifest_rel("src/actions")?)
				.xpipe(FileGroupToFuncTokens::default())?
				.xpipe(FuncTokensToAxumRoutes {
					codegen_file: CodegenFile::new(
						AbsPathBuf::new_manifest_rel_unchecked(
							"src/codegen/server_actions.rs",
						),
					)
					.with_pkg_name(&cx.pkg_name),
					..Default::default()
				})?
				.xmap(|(funcs, codegen)| -> Result<_> {
					codegen.build_and_write()?;
					Ok(funcs)
				})?;


		let pages = FileGroup::new(AbsPathBuf::new_manifest_rel("src/pages")?)
			.with_filter(
				GlobFilter::default()
					.with_include("*.rs")
					.with_exclude("*mod.rs"),
			)
			.xpipe(FileGroupToFuncTokens::default())?
			.xpipe(FuncTokensToRsxRoutes::new(
				CodegenFile::new(AbsPathBuf::new_manifest_rel_unchecked(
					"src/codegen/pages.rs",
				))
				.with_pkg_name(&cx.pkg_name),
			))?
			.xmap(|(funcs, codegen)| -> Result<_> {
				codegen.build_and_write()?;
				Ok(funcs)
			})?;

		let docs = FileGroup::new(AbsPathBuf::new_manifest_rel("src/docs")?)
			.xpipe(FileGroupToFuncTokens::default())?
			.xpipe(MapFuncTokens::default().base_route("/docs"))
			.xpipe(FuncTokensToRsxRoutes::new(
				CodegenFile::new(AbsPathBuf::new_manifest_rel_unchecked(
					"src/codegen/docs.rs",
				))
				.with_pkg_name(&cx.pkg_name),
			))?
			.xmap(|(funcs, codegen)| -> Result<_> {
				codegen.build_and_write()?;
				Ok(funcs)
			})?;





		FsExt::copy_recursive(
			AbsPathBuf::new_manifest_rel("../beet_design/public")?,
			AbsPathBuf::new_workspace_rel_unchecked("target/client"),
		)?;

		// ⚠️ this is a downstream copy of crates/beet_design/build.rs
		// we're actually only using the route paths, maybe we should generate
		// those in design
		let mockups =
			FileGroup::new(AbsPathBuf::new_manifest_rel("../beet_design/src")?)
				.with_filter(GlobFilter::default().with_include("*.mockup.*"))
				.xpipe(FileGroupToFuncTokens::default())?
				.xpipe(
					MapFuncTokens::default()
						.base_route("/design")
						.replace_route([(".mockup", "")]),
				);

		pages
			.funcs
			.xtend(mockups.funcs)
			.xtend(docs.funcs)
			.xinto::<FuncTokensTree>()
			.xpipe(FuncTokensTreeToRouteTree {
				codegen_file: CodegenFile::new(
					AbsPathBuf::new_manifest_rel_unchecked(
						"src/codegen/route_tree.rs",
					),
				)
				.with_pkg_name(&cx.pkg_name),
			})?;
	}
	Ok(())
}
