use beet::prelude::*;
use clap::Parser;
use heck::ToKebabCase;
use tokio::process::Command;



/// Run an sst subcommand in the infra directory.
#[derive(Parser)]
pub struct RunInfra {
	/// The subcommand to run (deploy or remove)
	#[arg(value_enum, default_value = "deploy")]
	subcommand: SstSubcommand,
	/// The stage to use
	#[arg(long)]
	stage: Option<String>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum SstSubcommand {
	Deploy,
	Remove,
}


impl RunInfra {
	pub fn new(subcommand: SstSubcommand) -> Self {
		RunInfra {
			subcommand,
			stage: None,
		}
	}
	pub fn stage(mut self, stage: impl ToString) -> Self {
		self.stage = Some(stage.to_string());
		self
	}

	/// The name of the lambda function matching the one
	/// in sst.config.ts -> new sst.aws.Function(`..`, {name: `THIS_FIELD` }),
	pub fn lambda_func_name(binary_name: &str) -> String {
		format! {"{}-lambda",binary_name.to_kebab_case()}
	}

	pub async fn run(&self) -> Result {
		let mut args = match &self.subcommand {
			SstSubcommand::Deploy => vec!["deploy"],
			SstSubcommand::Remove => vec!["remove"],
		};
		if let Some(stage) = &self.stage {
			args.push("--stage");
			args.push(stage);
		}

		let sst_dir = std::env::current_dir()?.join("infra").canonicalize()?;
		let mut cmd = Command::new("npx");
		cmd.arg("sst")
			// .arg("--config")
			// .arg("infra/sst.config.ts")
			.current_dir(sst_dir)
			.args(args);

		println!(
			"🌱 Running SST command: \n   {cmd:?}\n🌱 Interrupting this step may result in dangling AWS Resources"
		);
		cmd.status().await?.exit_ok()?.xok()
	}
}
