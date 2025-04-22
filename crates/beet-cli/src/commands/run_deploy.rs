use crate::prelude::*;
use anyhow::Result;
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
}


impl RunDeploy {
	/// Builds all required files and runs:
	/// - Build template map used by the binary
	/// - Build static files

	pub fn run(mut self) -> Result<()> {
		self.build.build_cmd.release = true;
		self.build.clone().into_group()?.run()?;

		self.lambda_build()?;
		if !self.dry_run {
			self.lambda_deploy()?;
		}
		Ok(())
	}

	fn lambda_build(&self) -> Result<()> {
		let mut cmd = Command::new("cargo");
		cmd.arg("lambda")
			.arg("build")
			.arg("--features")
			.arg("beet/lambda")
			.arg("--release");

		if let Some(pkg) = &self.build.build_cmd.package {
			cmd.arg("--package").arg(pkg);
		}

		cmd.spawn()?.wait()?.exit_ok()?;
		Ok(())
	}


	/// Deploy to lambda, using best effort to determine the binary name
	fn lambda_deploy(&self) -> Result<()> {
		let mut cmd = Command::new("cargo");

		let binary_name = if let Some(bin) = &self.build.build_cmd.bin {
			Some(bin)
		} else if let Some(pkg) = &self.build.build_cmd.package {
			Some(pkg)
		} else {
			None
		};

		cmd.arg("lambda")
			.arg("deploy")
			.arg("--enable-function-url")
			.arg("--include")
			.arg(&self.build.build_args.html_dir);

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

		cmd.spawn()?.wait()?.exit_ok()?;

		Ok(())
	}
}
