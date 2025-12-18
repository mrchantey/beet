use std::path::PathBuf;

use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_rsx::prelude::*;

/// Emitted for each stdout/stderr line. `is_err` is true for stderr.
#[derive(EntityTargetEvent)]
pub struct StdOutLine {
	pub line: String,
	pub is_err: bool,
}

/// Holds a handle to a spawned child process,
/// killed on drop.
/// This will only be inserted for a [`ChildProcess`] with `wait: false`
#[derive(Component)]
pub struct ChildHandle(async_process::Child);

impl Drop for ChildHandle {
	fn drop(&mut self) { self.0.kill().ok(); }
}

/// An action that will run the provided command asynchronously,
/// calling [`Outcome::Pass`] if the command was successful or [`Outcome::Fail`] if it failed.
/// All stdout and stderr lines are emitted as [`StdOutLine`] entity events.
///
/// Lines are streamed concurrently as they arrive. Empty lines are emitted (not skipped).
/// Reader tasks are aborted if the process exits first.
#[construct]
pub fn ChildProcess(
	/// The command to run
	cmd: String,
	current_dir: Option<PathBuf>,
	/// The command arguments
	args: Vec<String>,
	/// Environment variables to set
	envs: Vec<(String, String)>,
	/// Wait for the child to complete before triggering
	/// [`Outcome::Pass`], alternatively set false to pass immediately
	/// with the [`ChildHandle`] added.
	wait: bool,
	/// Kill any existing [`ChildHandle`] on this entity before running
	kill: bool,
) -> impl Bundle {
	OnSpawn::observe(
		move |mut ev: On<GetOutcome>,
		      config: Res<PackageConfig>,
		      mut commands: Commands|
		      -> Result {
			let cmd = cmd.clone();
			let current_dir = current_dir.clone();
			let args = args.clone();
			println!("Package config {config:#?}");
			let envs = envs.clone().xtend(config.envs());
			if kill {
				commands.entity(ev.action()).remove::<ChildHandle>();
			}
			ev.run_async(async move |mut action| {
				let envs_pretty = envs
					.iter()
					.map(|(k, v)| format!("{}={}", k, v))
					.collect::<Vec<_>>()
					.join(" ");
				info!("{} {} {}", envs_pretty, cmd, args.join(" "));
				// 1. spawn the command
				let mut cmd = async_process::Command::new(&cmd);
				cmd.args(&args).envs(envs);
				if let Some(dir) = current_dir {
					cmd.current_dir(dir);
				}
				let mut child = cmd.spawn()?;

				// 2. take stdout/stderr pipes
				// let stdout = child
				// 	.stdout
				// 	.take()
				// 	.ok_or_else(|| bevyhow!("stdout not found"))?;

				// let stderr = child
				// 	.stderr
				// 	.take()
				// 	.ok_or_else(|| bevyhow!("stderr not found"))?;

				let outcome = if wait {
					// wait for completion
					match child.status().await?.exit_ok() {
						Ok(_) => Outcome::Pass,
						Err(_) => Outcome::Fail,
					}
				} else {
					// pass immediately and store the child process
					action.entity().insert(ChildHandle(child)).await;
					Outcome::Pass
				};

				action.trigger_with_cx(outcome);

				Ok(())
			});
			Ok(())
		},
	)
}

impl Default for ChildProcess {
	fn default() -> Self {
		Self {
			cmd: "true".into(),
			args: default(),
			envs: default(),
			current_dir: None,
			wait: true,
			kill: true,
		}
	}
}

impl ChildProcess {
	pub fn new(cmd: impl Into<String>) -> Self {
		Self {
			cmd: cmd.into(),
			..default()
		}
	}

	pub fn command(mut self, cmd: impl Into<String>) -> Self {
		self.cmd = cmd.into();
		self
	}

	pub fn arg(mut self, arg: impl AsRef<str>) -> Self {
		self.args.push(arg.as_ref().to_string());
		self
	}

	pub fn current_dir(mut self, dir: impl Into<PathBuf>) -> Self {
		self.current_dir = Some(dir.into());
		self
	}

	pub fn env(
		mut self,
		key: impl Into<String>,
		value: impl Into<String>,
	) -> Self {
		self.envs.push((key.into(), value.into()));
		self
	}

	pub fn from_cargo(cargo: &CargoBuildCmd) -> Self {
		Self {
			cmd: "cargo".into(),
			args: cargo.get_args().iter().map(|s| s.to_string()).collect(),
			..default()
		}
	}

	/// Uses the first argument as the command and the remaining as args
	/// ## Panics
	/// Panics if the args are empty
	pub fn from_args(args: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
		let mut args = args.into_iter();
		let cmd = args
			.next()
			.expect("ChildProcess::from_args requires at least one argument")
			.as_ref()
			.to_string();
		Self {
			cmd,
			args: args.map(|s| s.as_ref().to_string()).collect(),
			..default()
		}
	}

	pub fn no_wait(mut self) -> Self {
		self.wait = false;
		self
	}
	pub fn no_kill(mut self) -> Self {
		self.kill = false;
		self
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_rsx::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, CliPlugin))
			.insert_resource(pkg_config!())
			.world_mut()
			.spawn(bsx! {
				<entity {(Sequence, ExitOnEnd)}>
					<ChildProcess
						cmd="echo"
						args=vec!["foobar".into()]
						default
						/>
				</entity>
			})
			.trigger_target(GetOutcome);

		app.run_async().await.xpect_eq(AppExit::Success);
	}
}
