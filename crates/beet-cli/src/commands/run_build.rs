use crate::prelude::*;
use beet::prelude::*;
use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;


/// Build the project
#[derive(Debug, Clone, Parser)]
pub struct RunBuild {
	/// 🦀 the commands that will be used to build the binary 🦀
	#[command(flatten)]
	build_cmd: CargoBuildCmd,
	/// Location of the beet.toml config file
	#[arg(long)]
	beet_config: Option<PathBuf>,
	/// Run a simple file server in this process instead of
	/// spinning up the native binary with the --server feature
	#[arg(long = "static")]
	r#static: bool,
	/// Only execute the provided build steps,
	/// options are `routes`, `static-scene`, `client-islands`
	#[arg(long, value_delimiter = ',', value_parser = parse_flags)]
	only: Vec<BuildFlag>,
}

fn parse_flags(s: &str) -> Result<BuildFlag, String> { BuildFlag::from_str(s) }


pub enum RunMode {
	Once,
	Watch,
}


impl RunBuild {
	pub async fn run(self, run_mode: RunMode) -> Result {
		let mut app = App::new();
		let config = BeetConfigFile::try_load_or_default::<BuildConfig>(
			self.beet_config.as_deref(),
		)
		.unwrap_or_exit();
		let cwd = config.template_config.workspace.root_dir.into_abs();
		let filter = config.template_config.workspace.filter.clone();

		let build_flags = if self.only.is_empty() {
			BuildFlags::All
		} else {
			BuildFlags::Only(self.only)
		};

		app.insert_resource(build_flags)
			.insert_resource(self.build_cmd)
			.add_non_send_plugin(config)
			.add_plugins(BuildPlugin::default());

		match run_mode {
			RunMode::Once => app.run_once(),
			RunMode::Watch => {
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
		.into_result()
	}
}
