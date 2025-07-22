use super::*;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_parse::prelude::ParseRsxTokensSequence;
use beet_rsx::as_beet::AbsPathBuf;
use beet_rsx::prelude::*;
use beet_utils::prelude::WatchEvent;
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
#[derive(Debug, Clone, Default)]
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

impl WorldSequence for BuildPlugin {
	fn run_sequence<R: WorldSequenceRunner>(
		self,
		runner: &mut R,
	) -> Result<()> {
		bevy::ecs::error::GLOBAL_ERROR_HANDLER
			.set(bevy::ecs::error::panic)
			.ok();

		#[cfg(not(test))]
		if !self.skip_load_workspace {
			load_workspace_source_files.run_sequence(runner)?;
		}
		(
			// style roundtrip breaks without resolving templates,
			// im not sure if this should be here, doesnt it indicate
			// we're relying on exprs in templates?
			// we should remove it!
			apply_static_rsx,
			// import step
			parse_file_watch_events,
			import_rsx_snippets_rs,
			import_rsx_snippets_md,
			// parse step
			|world: &mut World| {
				world.run_sequence_once(ParseRsxTokensSequence)?;
				Ok(())
			},
			update_file_expr_hash,
			|world: &mut World| {
				if world.resource::<BuildFlags>().contains(BuildFlag::Routes) {
					world.run_sequence_once(RouteCodegenSequence)?;
				}
				Ok(())
			},
		)
			.run_sequence(runner)?;

		if self.skip_write_to_fs {
			return Ok(());
		}

		let flags = runner.world().resource::<BuildFlags>().clone();
		if flags.contains(BuildFlag::Snippets) {
			export_snippets.run_sequence(runner)?;
		}
		if flags.contains(BuildFlag::Routes) {
			export_route_codegen.run_sequence(runner)?;
		}
		if flags.contains(BuildFlag::CompileServer) {
			compile_server.run_sequence(runner)?;
		}
		if flags.contains(BuildFlag::ExportSsg) {
			export_server_ssg.run_sequence(runner)?;
		}
		if flags.contains(BuildFlag::CompileWasm) {
			compile_client.run_sequence(runner)?;
		}
		if flags.contains(BuildFlag::RunServer) {
			run_server.run_sequence(runner)?;
		}
		Ok(())
	}
}

impl Plugin for BuildPlugin {
	fn build(&self, app: &mut App) {
		let this = self.clone();
		app.add_event::<WatchEvent>()
			.init_resource::<WorkspaceConfig>()
			.init_resource::<BuildFlags>()
			.init_resource::<ServerHandle>()
			.init_resource::<HtmlConstants>()
			.init_resource::<TemplateMacros>()
			// types
			.add_plugins(NodeTypesPlugin)
			// .add_plugins(TemplatePlugin)
			// .insert_resource(TemplateFlags::None)
			.add_systems(Update, move |world: &mut World| {
				world.run_sequence_once(this.clone())?;
				Ok(())
			});
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
			"compile-server" => Ok(BuildFlag::CompileServer),
			"export-ssg" => Ok(BuildFlag::ExportSsg),
			"compile-wasm" => Ok(BuildFlag::CompileWasm),
			"run-server" => Ok(BuildFlag::RunServer),
			_ => Err(format!("Unknown flag: {}", s)),
		}
	}
}
