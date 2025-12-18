use crate::prelude::*;
use beet_core::prelude::*;
use beet_rsx::prelude::*;
use clap::Parser;

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

#[construct]
pub fn CompileLambda(
	entity: Entity,
	query: AncestorQuery<&'static CargoBuildCmd>,
) -> impl Bundle {
	let mut cmd = query
		.get(entity)
		.cloned()
		.unwrap_or_default()
		// beet binaries should default to 'server' with 'native-tls' but we need
		// to disable that to specify 'deploy' feature
		.no_default_features()
		.feature("server-lambda")
		.xref()
		.xmap(ChildProcess::from_cargo);

	cmd.args.insert(0, "lambda".into());

	// Set CARGO_BUILD_TARGET to host architecture for proc-macro compatibility
	// This is needed when cross-compiling on ARM hosts (e.g., ARM Surface/Mac)
	// to ensure proc-macros are built for the host while the binary targets x86_64
	#[cfg(target_arch = "aarch64")]
	{
		cmd = cmd.env("CARGO_BUILD_TARGET", "aarch64-unknown-linux-gnu");
	}
	// debug!("🌱 Building lambda binary\n{:?}", cmd);

	// TODO we should support all lambda build features
	(
		Name::new("Compile Lambda"),
		cmd.arg("--lambda-dir")
			.arg("target/lambda/crates")
			.arg("--target")
			.arg("x86_64-unknown-linux-gnu"),
	)
}

/// Deploy to lambda, using best effort to determine the binary name
#[construct]
pub fn DeployLambda(
	workspace_config: Res<WorkspaceConfig>,
	lambda_config: Res<LambdaConfig>,
	pkg_config: Res<PackageConfig>,
) -> impl Bundle {
	let binary_name = pkg_config.binary_name();

	let snippets_dir = workspace_config
		.snippets_dir()
		// .into_abs()
		.to_string();

	let mut cmd = ChildProcess::new("cargo")
		.arg("lambda")
		.arg("deploy")
		.arg("--enable-function-url")
		// we dont include the html dir, thats uploaded to bucket
		.arg("--include")
		.arg(&snippets_dir)
		.arg("--lambda-dir")
		.arg("target/lambda/crates")
		.arg("--binary-name")
		.arg(binary_name);

	let vars = env_ext::vars_filtered(
		GlobFilter::default().with_include("OPENAI_API_KEY"),
	);
	if !vars.is_empty() {
		cmd = cmd.arg("--env-var").arg(
			vars.into_iter()
				.map(|(key, value)| format!("{key}={value}"))
				.collect::<Vec<_>>()
				.join(","),
		);
	}

	if let Some(iam_role) = &lambda_config.iam_role {
		cmd = cmd.arg("--iam-role").arg(iam_role);
	}
	if let Some(region) = &lambda_config.region {
		cmd = cmd.arg("--region").arg(region);
	};

	let lambda_name = pkg_config.router_lambda_name();
	cmd = cmd.arg(&lambda_name);

	// Print the full command before executing
	// println!("🌱 Deploying Lambda Binary to {lambda_name}\n   {cmd:?}");
	// TODO print command with redacted environment variables
	// println!("🌱 Deploying Lambda Binary to {lambda_name}");

	cmd
}



/// Listen for loggong messages from the router lambda,
/// this command never finishes.
#[construct]
pub fn WatchLambda(pkg_config: Res<PackageConfig>) -> impl Bundle {
	let lambda_name = pkg_config.router_lambda_name();
	(
		Name::new("Watch Lambda"),
		ChildProcess::new("aws")
			.arg("logs")
			.arg("tail")
			.arg(format!("/aws/lambda/{lambda_name}"))
			.arg("--format")
			.arg("short") // detailed,short,json
			.arg("--since")
			.arg("2m")
			.arg("--follow"),
	)
}
