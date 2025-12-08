use beet::prelude::*;
use clap::Parser;

// simple cargo generate for now
#[derive(Parser)]
pub struct NewCmd {
	/// Additional arguments to pass to cargo generate
	#[clap(last = true)]
	pub additional_args: Vec<String>,
}

impl NewCmd {
	pub async fn run(self) -> Result {
		todo!("update this")
		// let mut command = Command::new("cargo");
		// // TODO lock down to commit matching the cli release
		// command
		// 	.arg("generate")
		// 	.arg("--git")
		// 	.arg("https://github.com/mrchantey/beet")
		// 	.arg("crates/beet_new_web")
		// 	.args(&self.additional_args);

		// command.status().await?.exit_ok()?.xok()
	}
}
