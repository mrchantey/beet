use crate::prelude::*;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use sweet::prelude::*;

/// The default codegen builder for a beet site.
///
/// This will perform the following tasks:
///
/// - If a `src/actions` dir exists, generate server actions
/// - If a `src/pages` dir exists, generate pages codegen and add to the route tree
/// - If a `src/docs` dir exists, generate docs codegen and add to the route tree
///
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefaultSiteConfig {
	/// The name of the package being built, used for imports in codegen.
	#[serde(rename = "package_name")]
	pub pkg_name: String,
	#[serde(default = "default_src_path")]
	pub src_path: AbsPathBuf,
	/// Optionally set the path for the docs route.
	/// By default this is set to `/docs` but if your entire site is a docs
	/// site it may be more idiomatic to set this to `None`.
	#[serde(default = "default_docs_route")]
	pub docs_route: String,
	/// These imports will be added to the head of the wasm imports file.
	/// This will be required for any components with a client island directive.
	/// By default this will include `use beet::prelude::*;`
	#[serde(default = "default_wasm_imports", with = "syn_item_vec_serde")]
	pub wasm_imports: Vec<syn::Item>,
}
fn default_src_path() -> AbsPathBuf {
	AbsPathBuf::new_workspace_rel_unchecked("src")
}
fn default_docs_route() -> String { "/".to_string() }
fn default_wasm_imports() -> Vec<syn::Item> {
	vec![syn::parse_quote!(
		use beet::prelude::*;
	)]
}


impl DefaultSiteConfig {
	pub fn build_wasm(&self) -> Result<()> {
		CodegenFile {
			output: self.src_path.join("codegen/wasm.rs"),
			pkg_name: Some(self.pkg_name.clone()),
			imports: self.wasm_imports.clone(),
			..Default::default()
		}
		.xpipe(BuildWasmRoutes::default())?
		.xok()
	}

	/// the default setup for most beet projects.
	/// The [routes] param is for additional funcs to be added to the route tree
	// TODO expose various options
	pub fn build_native(&self, mut routes: Vec<FuncTokens>) -> Result<()> {
		// removing dir breaks the FsWatcher in live reload
		self.build_server_actions()?;


		if let Ok(pages_dir) = self.src_path.join_checked("pages") {
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
							self.src_path.join("codegen/pages.rs"),
						)
						.with_pkg_name(&self.pkg_name),
					))?
					.xmap(|(group, codegen)| -> Result<_> {
						codegen.build_and_write()?;
						Ok(group.funcs)
					})?,
			);
		}

		if let Ok(docs_dir) = self.src_path.join_checked("docs") {
			routes.extend(
				FileGroup::new(docs_dir)
					.xpipe(FileGroupToFuncTokens::default())?
					.xpipe(
						MapFuncTokens::default().base_route(&self.docs_route),
					)
					.xpipe(FuncTokensToRsxRoutes::new(
						CodegenFile::new(self.src_path.join("codegen/docs.rs"))
							.with_pkg_name(&self.pkg_name),
					))?
					.xmap(|(group, codegen)| -> Result<_> {
						codegen.build_and_write()?;
						Ok(group.funcs)
					})?,
			);
		}

		routes
			.xinto::<FuncTokensTree>()
			.xpipe(FuncTokensTreeToRouteTree {
				codegen_file: CodegenFile::new(
					self.src_path.join("codegen/route_tree.rs"),
				)
				.with_pkg_name(&self.pkg_name),
			})?;


		Ok(())
	}

	fn build_server_actions(&self) -> Result<()> {
		let Ok(actions_dir) = self.src_path.join_checked("actions") else {
			return Ok(());
		};

		let _client_actions = FileGroup::new(actions_dir.clone())
			.xpipe(FileGroupToFuncTokens::default())?
			.xmap(|g| g.into_tree())
			.xpipe(FuncTokensTreeToServerActions::new(
				CodegenFile::new(
					self.src_path.join("codegen/client_actions.rs"),
				)
				.with_pkg_name(&self.pkg_name),
			))?;
		let _server_actions = FileGroup::new(actions_dir)
			.xpipe(FileGroupToFuncTokens::default())?
			.xpipe(FuncTokensToAxumRoutes {
				codegen_file: CodegenFile::new(
					self.src_path.join("codegen/server_actions.rs"),
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
