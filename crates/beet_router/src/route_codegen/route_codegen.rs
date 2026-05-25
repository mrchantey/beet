//! Top-level codegen orchestration.
//!
//! A [`RouteCodegen`] groups one or more [`RouteCollection`]s with optional
//! typed-route-tree and client-action outputs. Scanning each collection once,
//! it emits the per-collection bundle files, the shared `routes::` module, and
//! the client-action callers.

use crate::prelude::*;
use beet_core::prelude::*;

/// Orchestrates a full codegen pass over a set of route collections.
#[derive(Default)]
pub struct RouteCodegen {
	/// The route collections to scan and emit.
	pub collections: Vec<RouteCollection>,
	/// Output for the typed `routes::` module, built from all pages collections.
	pub route_tree: Option<CodegenFile>,
	/// Output for client-action callers, built from all actions collections.
	pub client_actions: Option<CodegenFile>,
}

impl RouteCodegen {
	/// Creates an empty codegen pass.
	pub fn new() -> Self { Self::default() }

	/// Adds a collection to the pass.
	pub fn add_collection(mut self, collection: RouteCollection) -> Self {
		self.collections.push(collection);
		self
	}

	/// Sets the typed route-tree output.
	pub fn with_route_tree(mut self, codegen: CodegenFile) -> Self {
		self.route_tree = Some(codegen);
		self
	}

	/// Sets the client-action output.
	pub fn with_client_actions(mut self, codegen: CodegenFile) -> Self {
		self.client_actions = Some(codegen);
		self
	}

	/// Scans every collection and builds the populated codegen files without
	/// writing them to disk.
	pub async fn build(&self) -> Result<Vec<CodegenFile>> {
		let mut scanned = Vec::with_capacity(self.collections.len());
		for collection in &self.collections {
			scanned.push(collection.scan().await?);
		}

		let mut outputs = Vec::new();

		// per-collection bundle files
		for (collection, files) in self.collections.iter().zip(&scanned) {
			outputs.push(emit_collection(collection, files)?);
		}

		// typed route tree across all pages collections
		if let Some(codegen) = &self.route_tree {
			let mut codegen = codegen.clone();
			let route_paths = self
				.collections
				.iter()
				.zip(&scanned)
				.filter(|(collection, _)| {
					collection.category.include_in_route_tree()
				})
				.flat_map(|(_, files)| {
					files.iter().map(|file| file.route_path.clone())
				})
				.collect::<Vec<_>>();
			codegen.add_item(emit_route_tree(&route_paths)?);
			outputs.push(codegen);
		}

		// client-action callers across all actions collections
		if let Some(codegen) = &self.client_actions {
			let mut codegen = codegen.clone();
			let files = self
				.collections
				.iter()
				.zip(&scanned)
				.filter(|(collection, _)| {
					collection.category == RouteCollectionCategory::Actions
				})
				.flat_map(|(_, files)| files.iter().cloned())
				.collect::<Vec<_>>();
			for item in emit_client_actions(&files)? {
				codegen.add_item(item);
			}
			outputs.push(codegen);
		}

		Ok(outputs)
	}

	/// Runs the full codegen pass, writing every output file to disk.
	pub async fn export(self) -> Result<()> {
		for codegen in self.build().await? {
			codegen.build_and_write()?;
		}
		Ok(())
	}
}


// Codegen scans directories and writes files, so its end-to-end test only
// runs natively (wasm has no directory enumeration).
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use quote::ToTokens;

	fn site_dir(sub: &str) -> AbsPathBuf {
		AbsPathBuf::new_workspace_rel(format!(
			"crates/beet_router/tests/test_site/{sub}"
		))
		.unwrap()
	}

	fn codegen(name: &str) -> CodegenFile {
		CodegenFile::new(site_dir(&format!("codegen/{name}")))
	}

	fn test_codegen() -> RouteCodegen {
		RouteCodegen::new()
			.add_collection(RouteCollection::new(
				site_dir("pages"),
				codegen("pages.rs"),
			))
			.add_collection(RouteCollection::new(
				site_dir("content"),
				codegen("content.rs"),
			))
			.add_collection(
				RouteCollection::new(site_dir("actions"), codegen("actions.rs"))
					.with_category(RouteCollectionCategory::Actions),
			)
			.with_route_tree(codegen("route_tree.rs"))
			.with_client_actions(codegen("client_actions.rs"))
	}

	#[beet_core::test]
	async fn builds_all_outputs() {
		let outputs = test_codegen().build().await.unwrap();
		// pages, content, actions, route_tree, client_actions
		outputs.len().xpect_eq(5);
		outputs
			.iter()
			.map(|codegen| codegen.build_output().unwrap().to_token_stream())
			.map(|tokens| tokens.to_string())
			.collect::<Vec<_>>()
			.join("\n\n// ───────────────\n\n")
			.xpect_snapshot();
	}
}
