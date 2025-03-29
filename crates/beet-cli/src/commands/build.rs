use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use clap::Parser;

/// Build the project
#[derive(Debug, Parser)]
pub struct Build {
	/// 🦀 the commands that will be used to build the binary 🦀
	#[command(flatten)]
	pub build_cmd: BuildCmd,
	#[command(flatten)]
	pub watch_args: WatchArgs,
	#[command(flatten)]
	pub build_template_map: BuildTemplateMap,
}


impl Build {
	/// Builds all required files and runs:
	/// - Build template map used by the binary
	/// - Build static files

	pub fn run(&self) -> Result<()> {
		self.build_template_map.build_and_write()?;

		BuildStepGroup::default()
			.add(BuildNative::new(&self.build_cmd, &self.watch_args))
			.add(ExportStatic::new(
				&self.watch_args,
				&self.build_cmd.exe_path(),
			))
			.add(BuildWasm::new(&self.build_cmd, &self.watch_args)?)
			.run()?;
		Ok(())
	}
}
