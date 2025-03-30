use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use clap::Parser;

/// Build the project
#[derive(Debug, Clone, Parser)]
pub struct RunBuild {
	/// 🦀 the commands that will be used to build the binary 🦀
	#[command(flatten)]
	pub build_cmd: CargoBuildCmd,
	#[command(flatten)]
	pub watch_args: WatchArgs,
	#[command(flatten)]
	pub build_template_map: BuildTemplateMap,
	/// for use by watch command, inserts server after native build
	#[arg(long, default_value_t = false)]
	pub server: bool,
}


impl RunBuild {
	pub fn run(self) -> Result<()> { self.into_group()?.run() }

	pub fn into_group(self) -> Result<BuildStepGroup> {
		if self.watch_args.only.is_empty() {
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
			watch_args,
			build_template_map,
			server: _,
		} = self;
		for arg in watch_args.only.iter() {
			match arg.as_str() {
				"templates" => group.add(build_template_map.clone()),
				"native" => {
					group.add(BuildNative::new(&build_cmd, &watch_args))
				}
				"server" => group.add(RunServer::new(&watch_args, &exe_path)),
				"static" => {
					group.add(ExportStatic::new(&watch_args, &exe_path))
				}
				"wasm" => group.add(BuildWasm::new(&build_cmd, &watch_args)?),
				_ => anyhow::bail!("unknown build step: {}", arg),
			};
		}
		Ok(group)
	}
	fn into_group_default(self) -> Result<BuildStepGroup> {
		let Self {
			build_cmd,
			watch_args,
			build_template_map,
			server,
		} = self;


		let exe_path = build_cmd.exe_path();

		let mut group = BuildStepGroup::default()
			// 1. export the templates by statically viewing the files
			// 		recompile depends on a templates file existing
			// 		and build_templates doesnt depend on recompile so safe to do first
			.with(build_template_map)
			// 2. build the native binary
			.with(BuildNative::new(&build_cmd, &watch_args))
			// 3. export all static files from the app
			//   	- html files
			//   	- client island entries
			.with(ExportStatic::new(&watch_args, &exe_path));
		if server {
			group.add(RunServer::new(&watch_args, &exe_path));
		}
		// 4. build the wasm binary
		group.add(BuildWasm::new(&build_cmd, &watch_args)?);
		Ok(group)
	}
}
