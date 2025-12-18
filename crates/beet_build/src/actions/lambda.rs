use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
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
pub fn CompileLambda(entity: Entity) -> impl Bundle {
	(
		Name::new("Compile Lambda"),
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      mut cmd_params: CommandParams,
			      query: AncestorQuery<&'static CargoBuildCmd>| {
				let cargo_cmd = query
					.get(entity)
					.cloned()
					.unwrap_or_default()
					.cmd("build")
					// beet binaries should default to 'server' with 'native-tls' but we need
					// to disable that to specify 'deploy' feature
					.no_default_features()
					.feature("server-lambda");

				// Start with cargo lambda command
				let mut cmd_config = CommandConfig::new("cargo").arg("lambda");

				// Add the build args from CargoBuildCmd
				for arg in cargo_cmd.get_args() {
					cmd_config = cmd_config.arg(arg);
				}

				// Set CARGO_BUILD_TARGET to host architecture for proc-macro compatibility
				// This is needed when cross-compiling on ARM hosts (e.g., ARM Surface/Mac)
				// to ensure proc-macros are built for the host while the binary targets x86_64
				#[cfg(target_arch = "aarch64")]
				{
					config = config
						.env("CARGO_BUILD_TARGET", "aarch64-unknown-linux-gnu");
				}
				// debug!("🌱 Building lambda binary\n{:?}", config);

				// TODO we should support all lambda build features
				cmd_config = cmd_config
					.arg("--lambda-dir")
					.arg("target/lambda/crates")
					.arg("--target")
					.arg("x86_64-unknown-linux-gnu");

				cmd_params.execute(ev, cmd_config)
			},
		),
	)
}

/// Deploy to lambda, using best effort to determine the binary name
#[construct]
pub fn DeployLambda() -> impl Bundle {
	(
		Name::new("Deploy Lambda"),
		OnSpawn::observe(
			|ev: On<GetOutcome>,
			 mut cmd_params: CommandParams,
			 workspace_config: Res<WorkspaceConfig>,
			 lambda_config: Res<LambdaConfig>,
			 pkg_config: Res<PackageConfig>| {
				let binary_name = pkg_config.binary_name();

				let snippets_dir = workspace_config
					.snippets_dir()
					// .into_abs()
					.to_string();

				let mut config = CommandConfig::new("cargo")
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
					config = config.arg("--env-var").arg(
						vars.into_iter()
							.map(|(key, value)| format!("{key}={value}"))
							.collect::<Vec<_>>()
							.join(","),
					);
				}

				if let Some(iam_role) = &lambda_config.iam_role {
					config = config.arg("--iam-role").arg(iam_role);
				}
				if let Some(region) = &lambda_config.region {
					config = config.arg("--region").arg(region);
				};

				let lambda_name = pkg_config.router_lambda_name();
				config = config.arg(&lambda_name);

				// Print the full command before executing
				// println!("🌱 Deploying Lambda Binary to {lambda_name}\n   {config:?}");
				// TODO print command with redacted environment variables
				// println!("🌱 Deploying Lambda Binary to {lambda_name}");

				cmd_params.execute(ev, config)
			},
		),
	)
}



/// Listen for loggong messages from the router lambda,
/// this command never finishes.
#[construct]
pub fn WatchLambda() -> impl Bundle {
	(
		Name::new("Watch Lambda"),
		OnSpawn::observe(
			|ev: On<GetOutcome>,
			 mut cmd_params: CommandParams,
			 pkg_config: Res<PackageConfig>| {
				let lambda_name = pkg_config.router_lambda_name();
				let config = CommandConfig::new("aws")
					.arg("logs")
					.arg("tail")
					.arg(format!("/aws/lambda/{lambda_name}"))
					.arg("--format")
					.arg("short") // detailed,short,json
					.arg("--since")
					.arg("2m")
					.arg("--follow")
					// succeed immediatly to yield control for interruptions etc
					.no_wait();

				cmd_params.execute(ev, config)
			},
		),
	)
}
