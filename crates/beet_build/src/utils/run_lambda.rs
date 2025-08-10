use beet_core::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use clap::Parser;
use std::process::Command;



/// Deploy to AWS Lambda in release mode.
#[derive(Debug, Default, Clone, Parser, Resource)]
pub struct LambdaConfig {
	/// Specify the region to deploy the lambda function to
	#[arg(long)]
	pub region: Option<String>,
	/// Specify the IAM role that the lambda function should use
	#[arg(long)]
	pub iam_role: Option<String>,
}


pub fn compile_lambda(build_cmd: Res<CargoBuildCmd>) -> Result<()> {
	let build_cmd = build_cmd
		.clone()
		.cmd("build")
		// beet binaries should default to 'server' with 'native-tls' but we need
		// to disable that to specify 'deploy' feature
		.no_default_features()
		// force release, debug builds are generally way to big for lambda (450 MB > 65 MB)
		.release()
		.with_feature("deploy");

	println!("🌱 Compiling lambda binary");

	let mut cmd = Command::new("cargo");

	// TODO we should support all lambda build featire
	cmd.arg("lambda")
		.args(build_cmd.get_args())
		.arg("--lambda-dir")
		.arg("target/lambda/crates")
		.status()?
		.exit_ok()?
		.xok()
}

/// Deploy to lambda, using best effort to determine the binary name
pub fn deploy_lambda(
	workspace_config: Res<WorkspaceConfig>,
	lambda_config: Res<LambdaConfig>,
	infra: Res<InfraConfig>,
) -> Result {
	let binary_name = infra.binary_name();

	let html_dir = workspace_config
		.html_dir
		// .into_abs()
		.to_string();
	let snippets_dir = workspace_config
		.snippets_dir()
		// .into_abs()
		.to_string();


	let mut cmd = Command::new("cargo");
	cmd.arg("lambda")
		.arg("deploy")
		.arg("--enable-function-url")
		.arg("--include")
		.arg(&html_dir)
		.arg("--include")
		.arg(&snippets_dir)
		.arg("--lambda-dir")
		.arg("target/lambda/crates")
		.arg("--binary-name")
		.arg(&binary_name);

	if let Some(iam_role) = &lambda_config.iam_role {
		cmd.arg("--iam-role").arg(iam_role);
	}
	if let Some(region) = &lambda_config.region {
		cmd.arg("--region").arg(region);
	};

	let lambda_name = infra.default_lambda_name();
	cmd.arg(&lambda_name);

	// Print the full command before executing
	println!("🌱 Deploying Lambda Binary to {lambda_name}\n   {cmd:?}");

	cmd.status()?.exit_ok()?.xok()
}


pub fn lambda_log(infra: Res<InfraConfig>) -> Result {
	let mut cmd = Command::new("aws");
	let lambda_name = infra.default_lambda_name();
	println!("🌱 Watching Lambda logs {lambda_name}\n   {cmd:?}");
	cmd.arg("logs")
		.arg("tail")
		.arg(format!("/aws/lambda/{lambda_name}"))
		.arg("--format")
		.arg("short") // detailed,short,json
		.arg("--since")
		.arg("2m")
		.arg("--follow")
		.status()?
		.exit_ok()?
		.xok()
}


pub fn sync_bucket(
	ws_config: Res<WorkspaceConfig>,
	infra: Res<InfraConfig>,
) -> Result {
	let bucket_name = infra.default_bucket_name();

	let src = &ws_config.html_dir.into_abs().to_string();
	let dst = format!("s3://{bucket_name}");

	let mut cmd = Command::new("aws");
	cmd.arg("s3")
		.arg("sync")
		.arg(&src)
		.arg(&dst)
		.arg("--delete")
		.status()?
		.exit_ok()?
		.xok()
}
