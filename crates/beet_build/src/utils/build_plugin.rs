use crate::prelude::*;
use beet_core::prelude::*;
use beet_parse::prelude::*;
use beet_rsx::prelude::*;
use beet_utils::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use cargo_manifest::Manifest;
use std::str::FromStr;


#[derive(Resource, Deref)]
pub struct CargoManifest(Manifest);

impl CargoManifest {
	pub fn load() -> Result<CargoManifest> {
		Self::load_from_path(&AbsPathBuf::new_workspace_rel("Cargo.toml")?)
	}

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
	pub fn package_name(&self) -> Option<&str> {
		self.0.package.as_ref().map(|p| p.name.as_str())
	}
}

/// Main plugin for beet_build
#[derive(Debug, Default, Clone)]
pub struct BuildPlugin;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct BuildSequence;

impl Plugin for BuildPlugin {
	fn build(&self, app: &mut App) {
		bevy::ecs::error::GLOBAL_ERROR_HANDLER
			.set(bevy::ecs::error::panic)
			.ok();

		#[cfg(not(test))]
		app.insert_resource(
			//todo parse the package name etc, this is used for compiling server and client
			// but a verbatim parse() confuses the cli
			CargoBuildCmd::default(),
		)
		.insert_resource(CargoManifest::load().unwrap())
		.add_systems(
			Startup,
			// alternatively use import_route_file_collection to only load
			// source files used by file based routes
			load_workspace_source_files
				.run_if(BuildFlag::ImportSnippets.should_run()),
		);

		app.add_event::<WatchEvent>()
		.init_plugin(ParseRsxTokensPlugin)
		// .init_plugin(ApplyDirectivesPlugin)
			.init_plugin(RouteCodegenPlugin)
			.init_plugin(NodeTypesPlugin)
			.insert_schedule_before(Update, BuildSequence)
			.init_resource::<BuildFlags>()
			.init_resource::<WorkspaceConfig>()
			.init_resource::<ServerHandle>()
			.init_resource::<HtmlConstants>()
			.init_resource::<TemplateMacros>()
			.add_systems(
				BuildSequence,
				(
					// style roundtrip breaks without resolving templates,
					// im not sure if this should be here, doesnt it indicate
					// we're relying on exprs in templates?
					// we should remove it!
					apply_rsx_snippets,
					parse_file_watch_events,
					import_rsx_snippets_rs,
					import_rsx_snippets_md,
					ParseRsxTokens.run(),
					update_file_expr_hash,
					RouteCodegen.run(),
					export_snippets
						.run_if(BuildFlag::ExportSnippets.should_run()),
					export_route_codegen.run_if(BuildFlag::Routes.should_run()),
					compile_server
						.run_if(BuildFlag::CompileServer.should_run()),
					export_server_ssg.run_if(BuildFlag::ExportSsg.should_run()),
					compile_client.run_if(BuildFlag::CompileWasm.should_run()),
					run_server.run_if(BuildFlag::RunServer.should_run()),
				)
					.chain(),
			);
	}
}




#[derive(Debug, Default, Clone, PartialEq, Eq, Resource)]
pub enum BuildFlags {
	/// Run with all flags enabled.
	#[cfg_attr(not(test), default)]
	All,
	#[cfg_attr(test, default)]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildFlag {
	ImportSnippets,
	/// Generate File Snippet Scenes
	ExportSnippets,
	/// Generate Router Codegen
	Routes,
	CompileServer,
	ExportSsg,
	CompileWasm,
	RunServer,
}

impl BuildFlag {
	/// A predicate system for run_if conditions
	pub fn should_run(self) -> impl Fn(Res<BuildFlags>) -> bool {
		move |flags| flags.contains(self)
	}
}

impl std::fmt::Display for BuildFlag {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			BuildFlag::ImportSnippets => write!(f, "import-snippets"),
			BuildFlag::ExportSnippets => write!(f, "export-snippets"),
			BuildFlag::Routes => write!(f, "routes"),
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
			"import-snippets" => Ok(BuildFlag::ImportSnippets),
			"export-snippets" => Ok(BuildFlag::ExportSnippets),
			"routes" => Ok(BuildFlag::Routes),
			"compile-server" => Ok(BuildFlag::CompileServer),
			"export-ssg" => Ok(BuildFlag::ExportSsg),
			"compile-wasm" => Ok(BuildFlag::CompileWasm),
			"run-server" => Ok(BuildFlag::RunServer),
			_ => Err(format!("Unknown flag: {}", s)),
		}
	}
}
