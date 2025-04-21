use crate::prelude::*;
use beet_router::exports::syn::Item;
use beet_rsx::exports::anyhow::Result;
use beet_rsx::exports::sweet::prelude::*;

pub struct DefaultBuilder {
	/// The name of the package being built.
	/// By default this is set by `std::env::var("CARGO_PKG_NAME")`
	pub pkg_name: String,
	pub wasm_imports: Vec<Item>,
	/// Additional funcs to be added to the route tree
	pub funcs: Vec<FuncTokens>,
}


impl Default for DefaultBuilder {
	fn default() -> Self {
		Self {
			pkg_name: std::env::var("CARGO_PKG_NAME")
				.expect("DefaultBuilder: CARGO_PKG_NAME not set"),
			wasm_imports: Vec::new(),
			funcs: Vec::new(),
		}
	}
}


impl DefaultBuilder {
	pub fn build(self) -> Result<()> {
		println!("cargo::rerun-if-changed=build.rs");

		let is_wasm =
			std::env::var("TARGET").unwrap() == "wasm32-unknown-unknown";

		if is_wasm {
			CodegenFile {
				output: AbsPathBuf::new_manifest_rel_unchecked(
					"src/codegen/wasm.rs",
				),
				pkg_name: Some(self.pkg_name.clone()),
				imports: self.wasm_imports,
				..Default::default()
			}
			.xpipe(BuildWasmRoutes::default())?
		} else {
			// removing dir breaks the FsWatcher in live reload
			// FsExt::remove(&html_dir).ok();
			#[cfg(feature = "server")]
			self.build_server_actions()?;

			let pages =
				FileGroup::new(AbsPathBuf::new_manifest_rel("src/pages")?)
					.with_filter(
						GlobFilter::default()
							.with_include("*.rs")
							.with_exclude("*mod.rs"),
					)
					.xpipe(FileGroupToFuncTokens::default())?
					.xpipe(FuncTokensToRsxRoutes::new(
						CodegenFile::new(
							AbsPathBuf::new_manifest_rel_unchecked(
								"src/codegen/pages.rs",
							),
						)
						.with_pkg_name(&self.pkg_name),
					))?
					.xmap(|(funcs, codegen)| -> Result<_> {
						codegen.build_and_write()?;
						Ok(funcs)
					})?;

			let docs =
				FileGroup::new(AbsPathBuf::new_manifest_rel("src/docs")?)
					.xpipe(FileGroupToFuncTokens::default())?
					.xpipe(MapFuncTokens::default().base_route("/docs"))
					.xpipe(FuncTokensToRsxRoutes::new(
						CodegenFile::new(
							AbsPathBuf::new_manifest_rel_unchecked(
								"src/codegen/docs.rs",
							),
						)
						.with_pkg_name(&self.pkg_name),
					))?
					.xmap(|(funcs, codegen)| -> Result<_> {
						codegen.build_and_write()?;
						Ok(funcs)
					})?;


			pages
				.funcs
				.xtend(docs.funcs)
				.xtend(self.funcs)
				.xinto::<FuncTokensTree>()
				.xpipe(FuncTokensTreeToRouteTree {
					codegen_file: CodegenFile::new(
						AbsPathBuf::new_manifest_rel_unchecked(
							"src/codegen/route_tree.rs",
						),
					)
					.with_pkg_name(&self.pkg_name),
				})?;
		}

		Ok(())
	}

	#[cfg(feature = "server")]
	fn build_server_actions(&self) -> Result<()> {
		let _client_actions =
			FileGroup::new(AbsPathBuf::new_manifest_rel("src/actions")?)
				.xpipe(FileGroupToFuncTokens::default())?
				.xmap(|g| g.into_tree())
				.xpipe(FuncTokensTreeToServerActions::new(
					CodegenFile::new(AbsPathBuf::new_manifest_rel_unchecked(
						"src/codegen/client_actions.rs",
					))
					.with_pkg_name(&self.pkg_name),
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
					.with_pkg_name(&self.pkg_name),
					..Default::default()
				})?
				.xmap(|(funcs, codegen)| -> Result<_> {
					codegen.build_and_write()?;
					Ok(funcs)
				})?;
		Ok(())
	}
}
