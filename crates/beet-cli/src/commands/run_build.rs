use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use clap::Parser;
use std::path::PathBuf;

/// Build the project
#[derive(Debug, Clone, Parser)]
pub struct RunBuild {
	/// 🦀 the commands that will be used to build the binary 🦀
	#[command(flatten)]
	pub build_cmd: CargoBuildCmd,
	#[command(flatten)]
	pub build_args: BuildArgs,
	#[command(flatten)]
	pub build_template_map: BuildTemplateMap,
	/// used by watch command only, inserts server step after native build
	#[arg(long, default_value_t = false)]
	pub server: bool,
}

// TODO probably integrate with RunBuild, and just nest
#[derive(Debug, Clone, Parser)]
pub struct BuildArgs {
	/// Location of the beet.toml config file
	#[arg(long, default_value = "beet.toml")]
	pub config: PathBuf,
	/// Run a simple file server in this process instead of
	/// spinning up the native binary with the --server feature
	#[arg(long = "static")]
	pub as_static: bool,
	/// root for the emitted html files
	#[arg(long, default_value = "target/client")]
	pub html_dir: PathBuf,
	/// Only execute the provided build steps,
	/// options are `templates`, `native`, `server`, `static`, `wasm`
	#[arg(long, value_delimiter = ',')]
	pub only: Vec<String>,
}

impl RunBuild {
	pub fn run(self) -> Result<()> { self.into_group()?.run() }

	pub fn into_group(self) -> Result<BuildStepGroup> {
		if self.build_args.only.is_empty() {
			self.into_group_default()
		} else {
			self.into_group_custom()
		}
	}

	fn into_group_custom(self) -> Result<BuildStepGroup> {
		let mut group = BuildStepGroup::default();
		let exe_path = self.build_cmd.exe_path();
		let Self {
			build_cmd,
			build_args,
			build_template_map,
			server: _,
		} = self;
		for arg in build_args.only.iter() {
			match arg.as_str() {
				"templates" => group.add(build_template_map.clone()),
				"native-codegen" => group.add(BuildCodegenNative::new(&build_args)),
				"native" => {
					group.add(BuildNative::new(&build_cmd, &build_args))
				}
				"server" => group.add(RunServer::new(&build_args, &exe_path)),
				"static" => {
					group.add(ExportStatic::new(&build_args, &exe_path))
				}
				"wasm" => group.add(BuildWasm::new(&build_cmd, &build_args)?),
				_ => anyhow::bail!("unknown build step: {}", arg),
			};
		}
		Ok(group)
	}
	fn into_group_default(self) -> Result<BuildStepGroup> {
		let Self {
			build_cmd,
			build_args,
			build_template_map,
			server,
		} = self;


		let exe_path = build_cmd.exe_path();

		let mut group = BuildStepGroup::default()
			// 1. export the templates by statically viewing the files
			// 		recompile depends on a templates file existing
			// 		and build_templates doesnt depend on recompile so safe to do first
			.with(build_template_map)
			// 2. build native codegen
			.with(BuildCodegenNative::new(&build_args))
			// 2. build the native binary
			.with(BuildNative::new(&build_cmd, &build_args))
			// 3. export all static files from the app
			//   	- html files
			//   	- client island entries
			.with(ExportStatic::new(&build_args, &exe_path));
		if server {
			group.add(RunServer::new(&build_args, &exe_path));
		}
		// 4. build the wasm binary
		group.add(BuildWasm::new(&build_cmd, &build_args)?);
		Ok(group)
	}
}
