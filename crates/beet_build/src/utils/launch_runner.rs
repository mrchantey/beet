use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use clap::Parser;
use clap::Subcommand;
use dotenv::dotenv;
use std::str::FromStr;
use std::time::Duration;


/// Entrypoint for a beet launch sequence.
/// All environment variables in a `.env` file in the current directory will be loaded.
#[derive(Debug, Default, Clone, Parser)]
pub struct LaunchRunner {
	#[arg(short, long)]
	pub watch: bool,
	#[command(subcommand)]
	pub launch_cmd: Option<LaunchCmd>,
	/// Only execute the provided build steps, options are:
	/// - import-snippets
	/// - export-snippets
	/// - codegen
	/// - compile-server
	/// - compile-wasm
	/// - export-ssg
	/// - run-server
	/// - deploy-sst
	/// - compile-lambda
	/// - deploy-lambda
	#[arg(long, value_delimiter = ',', value_parser = parse_flags)]
	pub(crate) only: Vec<BuildFlag>,
	#[clap(flatten)]
	pub(crate) config_overrides: ConfigOverrides,
	#[clap(flatten)]
	pub(crate) lambda_config: LambdaConfig,
	#[arg(short, long)]
	pub package: Option<String>,
	// /// 🦀 the commands that will be used to build the binary 🦀
	// #[clap(flatten)]
	// pub(crate) build_cmd: CargoBuildCmd,
}

fn parse_flags(s: &str) -> Result<BuildFlag, String> { BuildFlag::from_str(s) }


impl LaunchRunner {
	pub fn runner(app: App) -> AppExit { Self::parse().run(app) }
	pub fn run(self, mut app: App) -> AppExit {
		dotenv().ok();
		PrettyTracing::default().init();
		app.add_plugins(self.config_overrides);
		let flags = if let Some(launch_cmd) = self.launch_cmd {
			launch_cmd.into()
		} else if self.only.is_empty() {
			LaunchCmd::Run.into()
		} else {
			BuildFlags::new(self.only)
		};
		app.insert_resource(flags);
		let mut build_cmd = CargoBuildCmd::default();
		build_cmd.package = self.package;
		app.insert_resource(build_cmd);
		app.insert_resource(self.lambda_config);

		let result = match self.watch {
			true => Self::watch(app),
			false => app.run_once(),
		};
		result
	}

	/// Run in watch mode, running again if any file
	/// specified in the [`WorkspaceConfig`] changes.
	#[tokio::main]
	async fn watch(mut app: App) -> AppExit {
		let config = app
			.init_resource::<WorkspaceConfig>()
			.world()
			.resource::<WorkspaceConfig>();
		let cwd = config.root_dir.into_abs();
		let filter = config.filter.clone();

		app.run_async(
			FsApp {
				watcher: FsWatcher {
					cwd: cwd.0,
					filter,
					debounce: Duration::from_millis(100),
				},
			}
			.runner(),
		)
		.await
	}
}

/// High level [`BuildFlag`] combinations
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum LaunchCmd {
	/// Exports snippets and codegen, then compiles and runs the server.
	#[default]
	Run,
	Codegen,
	Snippets,
	Serve,
	/// Build the client and server, export static html and syncs the s3 bucket.
	Static,
	/// Deploys the pulumi infra and lambda function.
	Deploy,
	/// Update the lambda function
	Lambda,
}

impl LaunchCmd {
	#[rustfmt::skip]
	pub fn into_flags(&self) -> Vec<BuildFlag> {
		match self {
			Self::Run => vec![]
				.xtend(Self::Codegen.into_flags())
				.xtend(Self::Snippets.into_flags())
				.xtend(vec![BuildFlag::CompileClient])
				.xtend(Self::Serve.into_flags()),
			Self::Snippets => vec![
				BuildFlag::ImportSnippets,
				BuildFlag::ExportSnippets,
			],
			Self::Codegen => vec![
				BuildFlag::ImportSnippets,
				BuildFlag::Codegen
			],
			Self::Serve => vec![
				BuildFlag::CompileServer,
				BuildFlag::ExportSsg,
				BuildFlag::RunServer,
			],
			Self::Static => vec![
				BuildFlag::ImportSnippets,
				BuildFlag::ExportSnippets,
				BuildFlag::CompileServer,
				BuildFlag::CompileClient,
				BuildFlag::ExportSsg,
				BuildFlag::SyncBucket,
			],
			Self::Deploy => vec![
				// BuildFlag::DeploySst,
				]
				.xtend(Self::Static.into_flags())
				.xtend(Self::Lambda.into_flags())
			,
			Self::Lambda => vec![
				BuildFlag::CompileLambda,
				BuildFlag::DeployLambda,
				BuildFlag::WatchLambda,
			],
		}
	}
}

impl Into<BuildFlags> for LaunchCmd {
	fn into(self) -> BuildFlags { BuildFlags::Only(self.into_flags()) }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Resource)]
pub enum BuildFlags {
	/// Run with all flags enabled.
	#[cfg_attr(not(test), default)]
	All,
	#[cfg_attr(test, default)]
	/// Run with no flags enabled.
	None,
	/// Only run with the specified flags.
	Only(Vec<BuildFlag>),
}

impl BuildFlags {
	pub fn new(mut flags: Vec<BuildFlag>) -> Self {
		if flags.is_empty() {
			Self::None
		} else {
			flags.sort();
			flags.dedup();
			Self::Only(flags)
		}
	}

	pub fn only(flag: BuildFlag) -> Self { Self::Only(vec![flag]) }
	pub fn contains(&self, flag: BuildFlag) -> bool {
		match self {
			Self::All => true,
			Self::None => false,
			Self::Only(flags) => flags.contains(&flag),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BuildFlag {
	/// Load all snippets defined in the [`WorkspaceConfig`]
	ImportSnippets,
	/// Generate File Snippet Scenes
	ExportSnippets,
	/// Generate Router Codegen
	Codegen,
	/// Compile the server binary
	CompileServer,
	/// Compile the wasm client binary
	CompileClient,
	/// Run the server to export static html, this re-runs on snippet changes
	ExportSsg,
	/// Run the server
	RunServer,
	/// Run `sst deploy`, syncing local config with cloud infra
	DeploySst,
	/// Build the lambda function
	CompileLambda,
	/// Deploy the lambda function
	DeployLambda,
	/// Watch the lambda function logs
	WatchLambda,
	/// Sync the s3 bucket
	SyncBucket,
}

impl BuildFlag {
	/// A predicate system for run_if conditions
	pub fn should_run(self) -> impl Fn(Res<BuildFlags>) -> bool {
		move |flags| flags.contains(self)
	}
}

impl std::fmt::Display for BuildFlag {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			BuildFlag::ImportSnippets => write!(f, "import-snippets"),
			BuildFlag::ExportSnippets => write!(f, "export-snippets"),
			BuildFlag::Codegen => write!(f, "codegen"),
			BuildFlag::CompileServer => write!(f, "compile-server"),
			BuildFlag::ExportSsg => write!(f, "export-ssg"),
			BuildFlag::CompileClient => write!(f, "compile-client"),
			BuildFlag::RunServer => write!(f, "run-server"),
			BuildFlag::DeploySst => write!(f, "deploy-sst"),
			BuildFlag::CompileLambda => write!(f, "compile-lambda"),
			BuildFlag::DeployLambda => write!(f, "deploy-lambda"),
			BuildFlag::WatchLambda => write!(f, "watch-lambda"),
			BuildFlag::SyncBucket => write!(f, "sync-bucket"),
		}
	}
}

impl FromStr for BuildFlag {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"import-snippets" => Ok(BuildFlag::ImportSnippets),
			"export-snippets" => Ok(BuildFlag::ExportSnippets),
			"codegen" => Ok(BuildFlag::Codegen),
			"compile-server" => Ok(BuildFlag::CompileServer),
			"export-ssg" => Ok(BuildFlag::ExportSsg),
			"compile-client" => Ok(BuildFlag::CompileClient),
			"run-server" => Ok(BuildFlag::RunServer),
			"deploy-sst" => Ok(BuildFlag::DeploySst),
			"compile-lambda" => Ok(BuildFlag::CompileLambda),
			"deploy-lambda" => Ok(BuildFlag::DeployLambda),
			"watch-lambda" => Ok(BuildFlag::WatchLambda),
			"sync-bucket" => Ok(BuildFlag::SyncBucket),
			_ => Err(format!("Unknown flag: {}", s)),
		}
	}
}
