use super::*;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_parse::prelude::ParseRsxTokensSequence;
use beet_rsx::as_beet::AbsPathBuf;
use beet_rsx::prelude::*;
use beet_utils::prelude::WatchEvent;
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
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
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

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BuildSequence;

/// for any [`Changed<SourceFile>`], import its rsx snippets as children,
/// then parse using [`ParseRsxTokens`].
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct ParseFileSnippetsSequence;

impl Plugin for ParseFileSnippetsSequence {
	fn build(&self, app: &mut App) {
		app.init_schedule(Self).add_systems(
			Self,
			(
				import_rsx_snippets_rs,
				import_rsx_snippets_md,
				// parse step
				ParseRsxTokensSequence.run(),
				update_file_expr_hash,
			)
				.chain(),
		);
	}
}

impl Plugin for BuildPlugin {
	fn build(&self, app: &mut App) {
		bevy::ecs::error::GLOBAL_ERROR_HANDLER
			.set(bevy::ecs::error::panic)
			.ok();
		#[allow(unused)]
		let Self {
			skip_load_workspace,
			skip_write_to_fs,
		} = self.clone();

		#[cfg(not(test))]
		if !skip_load_workspace {
			app.add_systems(Startup, load_workspace_source_files);
		}

		app.add_event::<WatchEvent>()
			.init_plugin(ParseRsxTokensSequence)
			.add_plugins((
				RouteCodegenSequence,
				ParseFileSnippetsSequence,
				NodeTypesPlugin,
			))
			.insert_schedule_before(Update, BuildSequence)
			.init_resource::<WorkspaceConfig>()
			.init_resource::<BuildFlags>()
			.init_resource::<ServerHandle>()
			.init_resource::<HtmlConstants>()
			.init_resource::<TemplateMacros>()
			// .add_plugins(TemplatePlugin)
			// .insert_resource(TemplateFlags::None)
			.add_systems(
				BuildSequence,
				(
					// style roundtrip breaks without resolving templates,
					// im not sure if this should be here, doesnt it indicate
					// we're relying on exprs in templates?
					// we should remove it!
					apply_rsx_snippets,
					ParseFileSnippetsSequence.run(),
					// import step
					parse_file_watch_events,
					RouteCodegenSequence
						.run()
						.run_if(BuildFlags::should_run(BuildFlag::Routes)),
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
						compile_client.run_if(BuildFlags::should_run(
							BuildFlag::CompileWasm,
						)),
						run_server.run_if(BuildFlags::should_run(
							BuildFlag::RunServer,
						)),
					)
						.chain()
						.run_if(move || !skip_write_to_fs),
				)
					.chain(),
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
