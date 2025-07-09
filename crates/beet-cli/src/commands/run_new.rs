use beet::prelude::*;
use clap::Parser;
use tokio::process::Command;

// simple cargo generate for now
#[derive(Parser)]
pub struct RunNew {
	/// Additional arguments to pass to cargo generate
	#[clap(last = true)]
	pub additional_args: Vec<String>,
}

impl RunNew {
	pub async fn run(self) -> Result {
		let mut command = Command::new("cargo");
		command
			.arg("generate")
			.arg("--git")
			.arg("https://github.com/mrchantey/beet")
			.arg("--path")
			.arg("crates/beet_new_web")
			.args(&self.additional_args);

		command.status().await?.exit_ok()?.xok()
	}
}
