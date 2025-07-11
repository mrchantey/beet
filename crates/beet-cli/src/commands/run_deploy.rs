use crate::prelude::*;
use beet::prelude::*;
use clap::Parser;
use tokio::process::Command;

/// Deploy to AWS Lambda in release mode.
#[derive(Debug, Parser)]
pub struct RunDeploy {
	#[command(flatten)]
	pub build: RunBuild,
	/// Specify the region to deploy the lambda function to
	#[arg(long)]
	pub region: Option<String>,
	/// use a specificed name for the lambda function,
	/// defaults to the package name. This must match sst.aws.Function(..,{name: THIS_FIELD })
	/// Specify the IAM role that the lambda function should use
	#[arg(long)]
	pub iam_role: Option<String>,
	/// Build but do not deploy
	#[arg(long)]
	pub dry_run: bool,
	/// Optionally specify the stage name used to match the lambda function name with
	/// the sst configuration. By default this is `dev` for debug builds and `prod` for release builds.
	#[arg(long)]
	pub stage: Option<String>,
}


impl RunDeploy {
	/// Builds all required files and runs:
	/// - Build template map used by the binary
	/// - Build static files

	pub async fn run(self) -> Result {
		// we need to build the native and wasm binaries,
		// and export the html
		let mut build = self.build.clone();
		build.build_cmd.release = true; // force release build
		build.run(RunMode::Once).await?;

		let config = BeetConfigFile::try_load_or_default::<BuildConfig>(
			self.build.beet_config.as_deref(),
		)
		.unwrap_or_exit();

		self.lambda_build().await?;
		if !self.dry_run {
			self.lambda_deploy(&config).await?;
		}
		Ok(())
	}

	async fn lambda_build(&self) -> Result<()> {
		let mut cmd = Command::new("cargo");
		// TODO we should support all lambda build featire
		cmd.arg("lambda")
			.arg("build")
			// beet binaries should default to 'server' with 'native-tls' but we need
			// to disable that to specify 'deploy' feature
			.arg("--no-default-features")
			// force release, debug builds are generally way to big for lambda (450 MB / 65 MB)
			.arg("--release")
			.arg("--features")
			.arg("deploy")
			.arg("--lambda-dir")
			.arg("target/lambda/crates");

		// if self.build.build_cmd.release {
		// 	cmd.arg("--release");
		// }
		if self.build.build_cmd.all_features {
			cmd.arg("--all-features");
		}
		if self.build.build_cmd.no_default_features {
			cmd.arg("--no-default-features");
		}
		if let Some(features) = &self.build.build_cmd.features {
			cmd.arg("--features").arg(features);
		}
		if let Some(pkg) = &self.build.build_cmd.package {
			cmd.arg("--package").arg(pkg);
		}
		if let Some(bin) = &self.build.build_cmd.bin {
			cmd.arg("--bin").arg(bin);
		}
		if let Some(example) = &self.build.build_cmd.example {
			cmd.arg("--example").arg(example);
		}
		if let Some(test) = &self.build.build_cmd.test {
			cmd.arg("--test").arg(test);
		}

		println!("🌱 Compiling lambda binary");
		cmd.status().await?.exit_ok()?.xok()
	}

	/// Deploy to lambda, using best effort to determine the binary name
	#[allow(unused)]
	async fn lambda_deploy(&self, config: &BuildConfig) -> Result {
		let mut cmd = Command::new("cargo");

		let binary_name = self.build.load_binary_name()?;

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
			.arg("--lambda-dir")
			.arg("target/lambda/crates")
			.arg("--binary-name")
			.arg(&binary_name);

		if let Some(iam_role) = &self.iam_role {
			cmd.arg("--iam-role").arg(iam_role);
		}
		if let Some(region) = &self.region {
			cmd.arg("--region").arg(region);
		};

		let stage = self
			.stage
			.as_ref()
			.map(|stage| stage.as_str())
			.unwrap_or_else(|| {
				if self.build.build_cmd.release {
					"prod"
				} else {
					"dev"
				}
			});

		let function_name = RunInfra::lambda_func_name(&binary_name, stage);
		cmd.arg(&function_name);

		// Print the full command before executing
		println!("🌱 Deploying Lambda Binary to {function_name}\n   {cmd:?}");

		cmd.status().await?.exit_ok()?.xok()
	}
}
