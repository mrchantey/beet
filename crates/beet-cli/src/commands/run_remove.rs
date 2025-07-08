// use crate::prelude::*;
use beet::prelude::*;
use clap::Parser;
use std::process::Command;



/// Remove all SST infrastructure
#[derive(Debug, Parser)]
pub struct RunRemove {}


impl RunRemove {
	pub async fn run(self) -> Result {
		println!(
			"🌱 Removing Infrastructure with SST \
🌱 Interrupting this step may result in dangling AWS Resources"
		);
		Command::new("npx")
			.arg("sst")
			.arg("remove")
			.arg("--config")
			.arg("infra/sst.config.ts")
			.spawn()?
			.wait()?
			.exit_ok()?
			.xok()
	}
}
