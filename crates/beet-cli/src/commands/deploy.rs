use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use clap::Parser;
use std::process::Command;

/// Deploy to AWS Lambda in release mode.
#[derive(Debug, Parser)]
pub struct Deploy {
	/// 🦀 the commands that will be used to build the binary 🦀
	#[command(flatten)]
	build_cmd: BuildCmd,
	#[command(flatten)]
	watch_args: WatchArgs,
	#[command(flatten)]
	build_template_map: BuildTemplateMap,
	/// Specify the region to deploy the lambda function to
	#[arg(long)]
	region: Option<String>,
	/// use a specificed name for the lambda function,
	/// defaults to the package name
	#[arg(long)]
	function_name: Option<String>,
	/// Specify the IAM role that the lambda function should use
	#[arg(long)]
	iam_role: Option<String>,
}


impl Deploy {
	/// Builds all required files and runs:
	/// - Build template map used by the binary
	/// - Build static files

	pub fn run(mut self) -> Result<()> {
		self.build_template_map.build_and_write()?;
		self.build_cmd.release = true;

		BuildStepGroup::default()
			.add(RunSetup::new(&self.build_cmd)?)
			.add(BuildNative::new(&self.build_cmd, &self.watch_args))
			.add(BuildWasm::new(&self.build_cmd, &self.watch_args)?)
			.add(BuildTemplates::new(
				&self.watch_args,
				&self.build_cmd.exe_path(),
			))
			.add(self)
			.run()?;

		Ok(())
	}

	fn lambda_build(&self) -> Result<()> {
		let mut cmd = Command::new("cargo");
		cmd.arg("lambda")
			.arg("build")
			.arg("--features")
			.arg("beet/lambda")
			.arg("--release");

		if let Some(pkg) = &self.build_cmd.package {
			cmd.arg("--package").arg(pkg);
		}

		cmd.spawn()?.wait()?.exit_ok()?;
		Ok(())
	}

	fn lambda_deploy(&self) -> Result<()> {
		let mut cmd = Command::new("cargo");

		let binary_name = if let Some(bin) = &self.build_cmd.bin {
			Some(bin)
		} else if let Some(pkg) = &self.build_cmd.package {
			Some(pkg)
		} else {
			None
		};

		cmd.arg("lambda")
			.arg("deploy")
			.arg("--enable-function-url")
			.arg("--include")
			.arg(&self.watch_args.html_dir);

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

impl BuildStep for Deploy {
	fn run(&self) -> Result<()> {
		self.lambda_build()?;
		self.lambda_deploy()?;
		Ok(())
	}
}
