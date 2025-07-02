use super::*;
use crate::prelude::*;
use beet_bevy::prelude::*;
use beet_common::prelude::*;
use beet_fs::process::WatchEvent;
use beet_parse::prelude::*;
use beet_template::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::str::FromStr;


/// Config file usually located at `beet.toml`
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildConfig {
	#[serde(flatten)]
	pub template_config: TemplateConfig,
	pub route_codegen: RouteCodegenConfig,
	pub collect_client_islands: CollectClientIslands,
}


impl NonSendPlugin for BuildConfig {
	fn build(self, app: &mut App) {
		app.add_non_send_plugin(self.route_codegen)
			.add_plugins(self.template_config);
		app.world_mut().spawn(self.collect_client_islands);
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
pub struct BeforeParseTokens;
/// Runs after rsx tokens have been resolved into entity trees,
/// and the new [`FileExprHash`] has been calculated.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct AfterParseTokens;
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
			.init_resource::<HtmlConstants>()
			.init_resource::<TemplateMacros>()
			// types
			.add_plugins((
				NodeTypesPlugin,
				ParseRsxTokensPlugin::default(),
				SnippetsPlugin::default(),
				RouteCodegenPlugin::default(),
				// ClientIslandCodegenPlugin::default(),
			))
			.configure_sets(
				Update,
				(
					BeforeParseTokens.before(ParseRsxTokensSet),
					AfterParseTokens
						.after(BeforeParseTokens)
						.after(ParseRsxTokensSet),
					ExportArtifactsSet
						.after(AfterParseTokens)
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
						spawn_templates,
						parse_file_watch_events,
						(parse_files_rs, parse_files_md),
					)
						.chain()
						.before(BeforeParseTokens),
					update_file_expr_hash
						.after(ParseRsxTokensSet)
						.before(AfterParseTokens),
					(
						export_codegen_files,
						compile_server,
						collect_client_islands,
						compile_wasm,
						run_server,
					)
						.chain()
						.in_set(ExportArtifactsSet),
				),
			);
	}
}


#[derive(Debug, Default, Clone, PartialEq, Eq, Resource)]
pub enum BuildFlags {
	/// Generate all build artifacts.
	#[default]
	All,
	/// Parse files but do not generate any output.
	None,
	/// Only generate the specified build artifacts.
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildFlag {
	/// Generate Router Codegen
	Routes,
	/// Generate File Snippet Scenes
	Snippets,
	/// Generate Client Islands Codegen
	ClientIslands,
	CompileServer,
	CompileWasm,
}


impl std::fmt::Display for BuildFlag {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			BuildFlag::Routes => write!(f, "routes"),
			BuildFlag::Snippets => write!(f, "snippets"),
			BuildFlag::ClientIslands => write!(f, "client-islands"),
			BuildFlag::CompileServer => write!(f, "compile-server"),
			BuildFlag::CompileWasm => write!(f, "compile-wasm"),
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
			"compile-wasm" => Ok(BuildFlag::CompileWasm),
			_ => Err(format!("Unknown only field: {}", s)),
		}
	}
}
