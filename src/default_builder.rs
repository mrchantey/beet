use crate::exports::syn;
use crate::prelude::*;
use beet_rsx::exports::anyhow::Result;
use beet_rsx::exports::sweet::prelude::*;


// build.rs Reference
// runtime env vars: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
// cargo:: output https://doc.rust-lang.org/cargo/reference/build-scripts.html#outputs-of-the-build-script



/// The default codegen builder for a beet site.
///
/// This will perform the following tasks:
///
/// - If a `src/actions` dir exists, generate server actions
/// - If a `src/pages` dir exists, generate pages codegen and add to the route tree
/// - If a `src/docs` dir exists, generate docs codegen and add to the route tree
///
pub struct DefaultBuilder {
	/// The name of the package being built.
	/// By default this is set by `std::env::var("CARGO_PKG_NAME")`
	pub pkg_name: String,
	/// These imports will be added to the head of the wasm imports file.
	/// This will be required for any components with a client island directive.
	/// By default this will include `use beet::prelude::*;`
	pub wasm_imports: Vec<syn::Item>,
	/// Additional funcs to be added to the route tree
	pub routes: Vec<FuncTokens>,
	/// Optionally set the path for the docs route.
	/// By default this is set to `/docs` but if your entire site is a docs
	/// site it may be more idiomatic to set this to `None`.
	pub docs_route: Option<String>,
}


impl Default for DefaultBuilder {
	fn default() -> Self {
		Self {
			pkg_name: std::env::var("CARGO_PKG_NAME")
				.expect("DefaultBuilder: CARGO_PKG_NAME not set"),
			wasm_imports: vec![syn::parse_quote!(
				use beet::prelude::*;
			)],
			routes: Vec::new(),
			docs_route: Some("/docs".to_string()),
		}
	}
}


impl DefaultBuilder {
	pub fn build(self) -> Result<()> {
		BuildUtils::rerun_if_changed("build.rs");
		BuildUtils::rerun_if_changed("src/pages/**");
		BuildUtils::rerun_if_changed("src/actions/**");
		BuildUtils::rerun_if_changed("src/docs/**");

		if BuildUtils::is_wasm() {
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

			let mut routes = self.routes;

			if let Ok(pages_dir) = AbsPathBuf::new_manifest_rel("src/pages") {
				routes.extend(
					FileGroup::new(pages_dir)
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
						.xmap(|(group, codegen)| -> Result<_> {
							codegen.build_and_write()?;
							Ok(group.funcs)
						})?,
				);
			}

			if let Ok(docs_dir) = AbsPathBuf::new_manifest_rel("src/docs") {
				routes.extend(
					FileGroup::new(docs_dir)
						.xpipe(FileGroupToFuncTokens::default())?
						.xpipe(MapFuncTokens::default().base_route(
							self.docs_route.unwrap_or_else(|| "/".to_string()),
						))
						.xpipe(FuncTokensToRsxRoutes::new(
							CodegenFile::new(
								AbsPathBuf::new_manifest_rel_unchecked(
									"src/codegen/docs.rs",
								),
							)
							.with_pkg_name(&self.pkg_name),
						))?
						.xmap(|(group, codegen)| -> Result<_> {
							codegen.build_and_write()?;
							Ok(group.funcs)
						})?,
				);
			}

			routes.xinto::<FuncTokensTree>().xpipe(
				FuncTokensTreeToRouteTree {
					codegen_file: CodegenFile::new(
						AbsPathBuf::new_manifest_rel_unchecked(
							"src/codegen/route_tree.rs",
						),
					)
					.with_pkg_name(&self.pkg_name),
				},
			)?;
		}

		Ok(())
	}

	#[cfg(feature = "server")]
	fn build_server_actions(&self) -> Result<()> {
		let Ok(actions_dir) = AbsPathBuf::new_manifest_rel("src/actions")
		else {
			return Ok(());
		};



		let _client_actions = FileGroup::new(actions_dir.clone())
			.xpipe(FileGroupToFuncTokens::default())?
			.xmap(|g| g.into_tree())
			.xpipe(FuncTokensTreeToServerActions::new(
				CodegenFile::new(AbsPathBuf::new_manifest_rel_unchecked(
					"src/codegen/client_actions.rs",
				))
				.with_pkg_name(&self.pkg_name),
			))?;
		let _server_actions = FileGroup::new(actions_dir)
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
