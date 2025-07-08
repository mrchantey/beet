use beet::prelude::*;
use clap::Parser;
use tokio::process::Command;

// simple cargo generate for now
#[derive(Parser)]
pub struct RunNew;

impl RunNew {
	pub async fn run(self) -> Result {
		Command::new("cargo")
			.arg("generate")
			.arg("mrchantey/beet_new_web")
			.arg("--name")
			.arg("beet_new_web")
			.arg("--force")
			.status()
			.await?
			.exit_ok()?
			.xok()
	}
}
