use super::*;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_parse::prelude::*;
use beet_rsx::as_beet::AbsPathBuf;
use beet_rsx::as_beet::ResultExtDisplay;
use beet_rsx::prelude::*;
use beet_utils::prelude::WatchEvent;
use bevy::prelude::*;
use cargo_manifest::Manifest;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use std::str::FromStr;

/// Config file usually located at `beet.toml`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildConfig {
	#[serde(flatten)]
	pub template_config: TemplateConfig,
	pub route_codegen: RouteCodegenConfig,
	pub client_island_codegen: CollectClientIslands,
	/// The path to the Cargo.toml file, defaults to `Cargo.toml`
	#[serde(default = "default_manifest_path")]
	manifest_path: PathBuf,
}

impl Default for BuildConfig {
	fn default() -> Self {
		Self {
			template_config: TemplateConfig::default(),
			route_codegen: RouteCodegenConfig::default(),
			client_island_codegen: CollectClientIslands::default(),
			manifest_path: default_manifest_path(),
		}
	}
}

impl BuildConfig {
	pub fn load_manifest(&self) -> Result<CargoManifest> {
		let path = AbsPathBuf::new(&self.manifest_path)?;
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
}


#[derive(Resource, Deref)]
pub struct CargoManifest(Manifest);

impl CargoManifest {
	pub fn package_name(&self) -> Option<&str> {
		self.0.package.as_ref().map(|p| p.name.as_str())
	}
}

fn default_manifest_path() -> PathBuf { PathBuf::from("Cargo.toml") }

impl NonSendPlugin for BuildConfig {
	fn build(self, app: &mut App) {
		let manifest = self.load_manifest().unwrap_or_exit();

		app.add_non_send_plugin(self.route_codegen)
			.add_plugins(self.template_config)
			.insert_resource(manifest);
		app.world_mut().spawn(self.client_island_codegen);
	}
}

/// Main plugin for beet_build
#[derive(Debug, Default)]
pub struct BuildPlugin {
	/// Disable loading the workspace source files, useful for
	/// testing or manually loading files.
	pub skip_load_workspace: bool,
	pub skip_write_to_fs: bool,
}
impl BuildPlugin {
	/// Do not read workspace files, and do not write any files to the filesystem.
	pub fn without_fs() -> Self {
		Self {
			skip_load_workspace: true,
			skip_write_to_fs: true,
		}
	}
}


/// Runs before rsx tokens have been resolved into entity trees,
/// used for importing and preparing token streams.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ImportSnippets;
/// Runs after rsx tokens have been resolved into entity trees,
/// and the new [`FileExprHash`] has been calculated.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ProcessChangedSnippets;
/// System set for exporting codegen files, Static Trees, Lang Partials, etc.
/// This set should be configured to run after all importing and processing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ExportArtifactsSet;



impl Plugin for BuildPlugin {
	fn build(&self, app: &mut App) {
		bevy::ecs::error::GLOBAL_ERROR_HANDLER
			.set(bevy::ecs::error::panic)
			.ok();

		if !self.skip_load_workspace {
			#[cfg(not(test))]
			app.add_systems(Startup, load_workspace_source_files);
		}
		let write_to_fs = !self.skip_write_to_fs;

		app.add_event::<WatchEvent>()
			.init_resource::<WorkspaceConfig>()
			.init_resource::<BuildFlags>()
			.init_resource::<ServerHandle>()
			.init_resource::<HtmlConstants>()
			.init_resource::<TemplateMacros>()
			// types
			.add_plugins((
				NodeTypesPlugin,
				ParseRsxTokensPlugin::default(),
				RouteCodegenPlugin::default(),
				// ClientIslandCodegenPlugin::default(),
			))
			.configure_sets(
				Update,
				(
					ImportSnippets.before(ParseRsxTokensSet),
					ProcessChangedSnippets
						.after(ImportSnippets)
						.after(ParseRsxTokensSet),
					ExportArtifactsSet
						.after(ProcessChangedSnippets)
						.before(TemplateSet)
						.run_if(move || write_to_fs),
				),
			)
			.add_systems(
				Update,
				(
					(
						// style roundtrip breaks without resolving templates,
						// im not sure if this should be here, doesnt it indicate
						// we're relying on exprs in templates?
						// we should remove it!
						apply_snippets_to_instances,
						parse_file_watch_events,
						(import_rsx_snippets_rs, import_rsx_snippets_md),
					)
						.chain()
						.before(ImportSnippets),
					update_file_expr_hash
						.after(ParseRsxTokensSet)
						.before(ProcessChangedSnippets),
					// compile and export steps
					(
						extract_lang_snippets,
						#[cfg(feature = "css")]
						parse_lightning,
					)
						.chain()
						.in_set(ProcessChangedSnippets)
						.run_if(BuildFlags::should_run(BuildFlag::Snippets)),
					(
						export_snippets.run_if(BuildFlags::should_run(
							BuildFlag::Snippets,
						)),
						export_route_codegen
							.run_if(BuildFlags::should_run(BuildFlag::Routes)),
						compile_server.run_if(BuildFlags::should_run(
							BuildFlag::CompileServer,
						)),
						export_server_ssg.run_if(BuildFlags::should_run(
							BuildFlag::ExportSsg,
						)),
						export_client_island_codegen.run_if(
							BuildFlags::should_run(BuildFlag::ClientIslands),
						),
						compile_client.run_if(BuildFlags::should_run(
							BuildFlag::CompileWasm,
						)),
						run_server.run_if(BuildFlags::should_run(
							BuildFlag::RunServer,
						)),
					)
						.chain()
						.in_set(ExportArtifactsSet),
				),
			);
	}
}


#[derive(Debug, Default, Clone, PartialEq, Eq, Resource)]
pub enum BuildFlags {
	/// Run with all flags enabled.
	#[default]
	All,
	/// Run with no flags enabled.
	None,
	/// Only run with the specified flags.
	Only(Vec<BuildFlag>),
}

impl BuildFlags {
	pub fn only(flag: BuildFlag) -> Self { Self::Only(vec![flag]) }
	pub fn contains(&self, flag: BuildFlag) -> bool {
		match self {
			Self::All => true,
			Self::None => false,
			Self::Only(flags) => flags.contains(&flag),
		}
	}

	/// A predicate system for run_if conditions
	pub fn should_run(flag: BuildFlag) -> impl Fn(Res<Self>) -> bool {
		move |flags| flags.contains(flag)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildFlag {
	/// Generate Router Codegen
	Routes,
	/// Generate File Snippet Scenes
	Snippets,
	/// Generate Client Islands Codegen
	ClientIslands,
	CompileServer,
	ExportSsg,
	CompileWasm,
	RunServer,
}


impl std::fmt::Display for BuildFlag {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			BuildFlag::Routes => write!(f, "routes"),
			BuildFlag::Snippets => write!(f, "snippets"),
			BuildFlag::ClientIslands => write!(f, "client-islands"),
			BuildFlag::CompileServer => write!(f, "compile-server"),
			BuildFlag::ExportSsg => write!(f, "export-ssg"),
			BuildFlag::CompileWasm => write!(f, "compile-wasm"),
			BuildFlag::RunServer => write!(f, "run-server"),
		}
	}
}

impl FromStr for BuildFlag {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"routes" => Ok(BuildFlag::Routes),
			"snippets" => Ok(BuildFlag::Snippets),
			"client-islands" => Ok(BuildFlag::ClientIslands),
			"compile-server" => Ok(BuildFlag::CompileServer),
			"export-ssg" => Ok(BuildFlag::ExportSsg),
			"compile-wasm" => Ok(BuildFlag::CompileWasm),
			"run-server" => Ok(BuildFlag::RunServer),
			_ => Err(format!("Unknown flag: {}", s)),
		}
	}
}
