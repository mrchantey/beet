use crate::prelude::*;
use beet::prelude::*;
use clap::Parser;
use std::process::Command;

/// Deploy to AWS Lambda in release mode.
#[derive(Debug, Parser)]
pub struct RunDeploy {
	#[command(flatten)]
	pub build: RunBuild,
	/// Specify the region to deploy the lambda function to
	#[arg(long)]
	pub region: Option<String>,
	/// use a specificed name for the lambda function,
	/// defaults to the package name
	#[arg(long)]
	pub function_name: Option<String>,
	/// Specify the IAM role that the lambda function should use
	#[arg(long)]
	pub iam_role: Option<String>,
	/// Build but do not deploy
	#[arg(long)]
	pub dry_run: bool,
	/// Also deploy sst infrastructure using a config file
	/// located at `infra/sst.config.ts`
	#[arg(long)]
	pub sst: bool,
}


impl RunDeploy {
	/// Builds all required files and runs:
	/// - Build template map used by the binary
	/// - Build static files

	pub async fn run(mut self) -> Result {
		self.build.build_cmd.release = true;
		// we need to build the native and wasm binaries in release mode,
		// and export the html
		self.build.clone().run(RunMode::Once).await?;

		let config = BeetConfigFile::try_load_or_default::<BuildConfig>(
			self.build.beet_config.as_deref(),
		)
		.unwrap_or_exit();

		self.lambda_build()?;
		if !self.dry_run {
			if self.sst {
				self.sst_deploy()?;
			}
			self.lambda_deploy(&config)?;
		}
		Ok(())
	}

	fn lambda_build(&self) -> Result<()> {
		let mut cmd = Command::new("cargo");
		cmd.arg("lambda")
			.arg("build")
			.arg("--no-default-features")
			.arg("--features")
			.arg("deploy")
			.arg("--release")
			// this is where sst expects the boostrap to be located
			.arg("--lambda-dir")
			.arg("target/lambda/crates");

		if let Some(pkg) = &self.build.build_cmd.package {
			cmd.arg("--package").arg(pkg);
		}
		println!("🌱 Compiling lambda binary");
		cmd.spawn()?.wait()?.exit_ok()?;
		Ok(())
	}
	fn sst_deploy(&self) -> Result<()> {
		println!(
			"🌱 Deploying Infrastructure with SST \
🌱 Interrupting this step may result in dangling AWS Resources"
		);
		Command::new("npx")
			.arg("sst")
			.arg("deploy")
			.arg("--stage")
			.arg("production")
			.arg("--config")
			.arg("infra/sst.config.ts")
			.spawn()?
			.wait()?
			.exit_ok()?
			.xok()
	}

	/// Deploy to lambda, using best effort to determine the binary name
	#[allow(unused)]
	fn lambda_deploy(&self, config: &BuildConfig) -> Result<()> {
		let mut cmd = Command::new("cargo");

		let binary_name = if let Some(bin) = &self.build.build_cmd.bin {
			Some(bin)
		} else if let Some(pkg) = &self.build.build_cmd.package {
			Some(pkg)
		} else {
			None
		};

		let html_dir = config
			.template_config
			.workspace
			.html_dir
			// .into_abs()
			.to_string();
		let snippets_dir = config
			.template_config
			.workspace
			.snippets_dir()
			// .into_abs()
			.to_string();


		cmd.arg("lambda")
			.arg("deploy")
			.arg("--enable-function-url")
			.arg("--include")
			.arg(&html_dir)
			.arg("--include")
			.arg(&snippets_dir)
			// this is where sst expects the boostrap to be located
			.arg("--lambda-dir")
			.arg("target/lambda/crates");

		if let Some(bin) = &binary_name {
			cmd.arg("--binary-name").arg(&bin);
		}

		if let Some(iam_role) = &self.iam_role {
			cmd.arg("--iam-role").arg(iam_role);
		}
		if let Some(region) = &self.region {
			cmd.arg("--region").arg(region);
		};

		if let Some(name) = &self.function_name {
			cmd.arg(name);
		}

		// Print the full command before executing
		let cmd_str = format!(
			"cargo {}",
			cmd.get_args()
				.map(|a| a.to_string_lossy())
				.collect::<Vec<_>>()
				.join(" ")
		);
		println!("🌱 Deploying Lambda Binary: {cmd_str}");

		cmd.spawn()?.wait()?.exit_ok()?;

		Ok(())
	}
}
