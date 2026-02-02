//! Main build plugin and schedule definitions.
//!
//! This module provides the [`BuildPlugin`] that sets up the entire build system,
//! including source file parsing, RSX token processing, and route code generation.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use beet_parse::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
use cargo_manifest::Manifest;

/// Wrapper around a cargo manifest for accessing package metadata.
#[derive(Resource, Deref)]
pub struct CargoManifest(Manifest);

impl CargoManifest {
	/// Loads the cargo manifest from the workspace root.
	pub fn load() -> Result<CargoManifest> {
		Self::load_from_path(&AbsPathBuf::new_workspace_rel("Cargo.toml")?)
	}

	/// Loads a cargo manifest from a specific path.
	pub fn load_from_path(path: &AbsPathBuf) -> Result<CargoManifest> {
		Manifest::from_path(&path)
			.map_err(|e| {
				bevyhow!(
					"Failed to load Cargo manifest\nPath:{}\nError:{}",
					path,
					e
				)
			})
			.map(|manifest| CargoManifest(manifest))
	}

	/// Returns the package name from the manifest, if available.
	pub fn package_name(&self) -> Option<&str> {
		self.0.package.as_ref().map(|p| p.name.as_str())
	}
}

/// Main plugin for the beet build system.
///
/// This plugin sets up all necessary resources and schedules for:
/// - Source file watching and management
/// - RSX token parsing and processing
/// - Route code generation
/// - Snippet export
#[derive(Debug, Default, Clone)]
pub struct BuildPlugin;


/// Schedule for loading and parsing source files in a workspace.
///
/// This schedule:
/// 1. Converts source files to an ECS representation using [`ParseRsxTokens`]
/// 2. Updates file expression hashes for change detection
/// 3. Runs route codegen
/// 4. Exports RSX snippets and codegen files
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct ParseSourceFiles;

impl Plugin for BuildPlugin {
	fn build(&self, app: &mut App) {
		app.try_set_error_handler(bevy::ecs::error::panic);
		app.init_plugin::<ParseRsxTokensPlugin>()
			.init_plugin::<RouteCodegenPlugin>()
			.init_plugin::<NodeTypesPlugin>()
			.insert_resource(CargoManifest::load().unwrap())
			.init_resource::<WorkspaceConfig>()
			.init_resource::<LambdaConfig>()
			.init_resource::<HtmlConstants>()
			.init_resource::<TemplateMacros>()
			.add_observer(parse_dir_watch_events)
			.add_systems(
				ParseRsxTokens,
				import_file_inner_text.in_set(ModifyRsxTree),
			)
			.add_systems(
				ParseSourceFiles,
				(
					reparent_route_collection_source_files,
					import_rsx_snippets_rs,
					import_rsx_snippets_md,
					ParseRsxTokens::as_system(),
					update_file_expr_hash,
					RouteCodegen::as_system(),
					export_snippets,
					export_codegen,
				)
					.chain(),
			);
	}
}
