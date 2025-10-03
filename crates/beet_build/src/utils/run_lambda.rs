use beet_core::prelude::*;
use clap::Parser;
use std::process::Command;



/// Deploy to AWS Lambda in release mode.
// TODO infra as entities
#[derive(Debug, Default, Clone, Parser, Resource)]
pub struct LambdaConfig {
	/// A list of environment variables to pass to the lambda function
	// #[clap(flatten)]
	// pub env_filter: GlobFilter,
	/// Specify the region to deploy the lambda function to
	#[arg(long)]
	pub region: Option<String>,
	/// Specify the IAM role that the lambda function should use
	#[arg(long)]
	pub iam_role: Option<String>,
}


pub fn compile_lambda(
	build_cmd: Res<CargoBuildCmd>,
	pkg_config: Res<PackageConfig>,
) -> Result {
	let build_cmd = build_cmd
		.clone()
		.cmd("build")
		// force release, debug builds are generally way to big (450 MB) for lambda ( max 65 MB)
		.release()
		// beet binaries should default to 'server' with 'native-tls' but we need
		// to disable that to specify 'deploy' feature
		.no_default_features()
		.with_feature("server-lambda");

	let mut cmd = Command::new("cargo");

	// TODO we should support all lambda build featire
	cmd.arg("lambda")
		.envs(pkg_config.envs())
		.args(build_cmd.get_args())
		.arg("--lambda-dir")
		.arg("target/lambda/crates")
		.xtap(|cmd| {
			debug!("🌱 Building lambda binary\n{:?}", cmd);
		})
		.status()?
		.exit_ok()?
		.xok()
}

/// Deploy to lambda, using best effort to determine the binary name
pub fn deploy_lambda(
	workspace_config: Res<WorkspaceConfig>,
	lambda_config: Res<LambdaConfig>,
	pkg_config: Res<PackageConfig>,
) -> Result {
	let binary_name = pkg_config.binary_name();

	let snippets_dir = workspace_config
		.snippets_dir()
		// .into_abs()
		.to_string();

	let mut cmd = Command::new("cargo");
	cmd.arg("lambda")
		.arg("deploy")
		.arg("--enable-function-url")
		// we dont include the html dir, thats uploaded to bucket
		.arg("--include")
		.arg(&snippets_dir)
		.arg("--lambda-dir")
		.arg("target/lambda/crates")
		.arg("--binary-name")
		.arg(&binary_name);

	let vars = env_ext::vars_filtered(
		GlobFilter::default().with_include("OPENAI_API_KEY"),
	);
	if !vars.is_empty() {
		cmd.arg("--env-var").arg(
			vars.into_iter()
				.map(|(key, value)| format!("{key}={value}"))
				.collect::<Vec<_>>()
				.join(","),
		);
	}

	if let Some(iam_role) = &lambda_config.iam_role {
		cmd.arg("--iam-role").arg(iam_role);
	}
	if let Some(region) = &lambda_config.region {
		cmd.arg("--region").arg(region);
	};

	let lambda_name = pkg_config.router_lambda_name();
	cmd.arg(&lambda_name);

	// Print the full command before executing
	// println!("🌱 Deploying Lambda Binary to {lambda_name}\n   {cmd:?}");
	// TODO print command with redacted environment variables
	println!("🌱 Deploying Lambda Binary to {lambda_name}");

	cmd.status()?.exit_ok()?.xok()
}


pub fn lambda_log(pkg_config: Res<PackageConfig>) -> Result {
	let mut cmd = Command::new("aws");
	let lambda_name = pkg_config.router_lambda_name();
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
