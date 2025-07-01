use crate::prelude::*;
use beet::prelude::*;
use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;


/// Build the project
#[derive(Debug, Clone, Parser)]
pub struct RunBuild {
	/// 🦀 the commands that will be used to build the binary 🦀
	#[command(flatten)]
	build_cmd: CargoBuildCmd,
	/// Determine the config location and which builds steps to run
	#[command(flatten)]
	build_args: BuildArgs,
}

impl RunBuild {
	/// Run once
	pub async fn build(self) -> Result {
		App::new()
			.insert_resource(self.build_cmd)
			.add_plugins(self.build_args)
			.run_once()
			.into_result()
	}


	/// Run in watch mode with a file watcher
	pub async fn run(self) -> Result {
		App::new()
			.insert_resource(self.build_cmd)
			.add_plugins(self.build_args)
			.run_async(FsApp::default().runner())
			.await
			.into_result()
	}
}



#[derive(Debug, Clone, Parser)]
struct BuildArgs {
	/// Location of the beet.toml config file
	#[arg(long)]
	beet_config: Option<PathBuf>,
	/// Run a simple file server in this process instead of
	/// spinning up the native binary with the --server feature
	#[arg(long = "static")]
	r#static: bool,
	/// Only execute the provided build steps,
	/// options are `templates`, `routes`, `server`, `static`, `client-islands`
	#[arg(long, value_delimiter = ',', value_parser = parse_build_only)]
	only: Vec<BuildOnly>,
}

fn parse_build_only(s: &str) -> Result<BuildOnly, String> {
	BuildOnly::from_str(s)
}
/// Insert resources and plugins to reflect the [`only`] options,
/// inserting all if [`only`] is empty.
impl Plugin for BuildArgs {
	fn build(&self, app: &mut App) {
		let config = BeetConfigFile::try_load_or_default::<BuildConfig>(
			self.beet_config.as_deref(),
		)
		.unwrap_or_exit();

		app.add_plugins((
			config.template_config,
			ParseRsxTokensPlugin::default(),
			ExportArtifactsPlugin::default(),
		));

		// selectively load plugins
		let all = self.only.is_empty();

		if all || self.only.contains(&BuildOnly::Routes) {
			app.add_plugins(RouteCodegenPlugin::default())
				.add_non_send_plugin(config.route_codegen);
		}
		if all || self.only.contains(&BuildOnly::StaticScene) {
			app.add_plugins(StaticScenePlugin::default());
		}
		if all || self.only.contains(&BuildOnly::ClientIslands) {
			app.add_plugins(ClientIslandCodegenPlugin::default())
				.add_non_send_plugin(config.client_island_codegen);
		}
	}
}


#[derive(Debug, Clone, PartialEq, Eq)]
enum BuildOnly {
	/// Router codegen
	Routes,
	StaticScene,
	ClientIslands,
}


impl std::fmt::Display for BuildOnly {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			BuildOnly::Routes => write!(f, "routes"),
			BuildOnly::StaticScene => write!(f, "static-scene"),
			BuildOnly::ClientIslands => write!(f, "client-islands"),
		}
	}
}

impl FromStr for BuildOnly {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"routes" => Ok(BuildOnly::Routes),
			"static-scene" => Ok(BuildOnly::StaticScene),
			"client-islands" => Ok(BuildOnly::ClientIslands),
			_ => Err(format!("Unknown only field: {}", s)),
		}
	}
}
