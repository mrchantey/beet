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

/// Configuration for command actions
#[derive(Debug, Clone)]
pub struct CommandConfig {
	/// The command to run
	cmd: String,
	/// The command arguments
	args: Vec<String>,
	/// Current working directory
	current_dir: Option<PathBuf>,
	/// Environment variables to set, in addition to
	/// [`PackageConfig::envs`]
	envs: Vec<(String, String)>,
	/// Wait for the child to complete before triggering
	/// [`Outcome::Pass`], alternatively set false to pass immediately
	/// with the [`ChildHandle`] added.
	wait: bool,
	/// Kill any existing [`ChildHandle`] on this entity before running
	kill: bool,
}

impl CommandConfig {
	/// Accepts a full command string, and splits into
	/// parts via `split_whitespace`
	pub fn parse(full_cmd: impl Into<String>) -> Self {
		let cmd = full_cmd.into();
		let mut args = cmd.split_whitespace();
		let cmd = args
			.next()
			.expect("RawCommand::new requires at least one argument")
			.to_string();
		Self {
			cmd,
			args: args.map(|s| s.to_string()).collect(),
			..default()
		}
	}


	pub fn from_parts(
		cmd: impl AsRef<str>,
		args: impl IntoIterator<Item = impl AsRef<str>>,
	) -> Self {
		Self {
			cmd: cmd.as_ref().to_string(),
			args: args.into_iter().map(|s| s.as_ref().to_string()).collect(),
			..default()
		}
	}

	pub fn arg(mut self, arg: impl AsRef<str>) -> Self {
		self.args.push(arg.as_ref().to_string());
		self
	}
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

	pub fn no_wait(mut self) -> Self {
		self.wait = false;
		self
	}
	pub fn no_kill(mut self) -> Self {
		self.kill = false;
		self
	}

	pub fn from_cargo(cargo: &CargoBuildCmd) -> Self {
		Self {
			cmd: "cargo".into(),
			args: cargo.get_args().iter().map(|s| s.to_string()).collect(),
			..default()
		}
	}
}

impl Default for CommandConfig {
	fn default() -> Self {
		Self {
			cmd: "true".into(),
			args: default(),
			current_dir: None,
			envs: default(),
			wait: true,
			kill: true,
		}
	}
}

impl From<CargoBuildCmd> for CommandConfig {
	fn from(cargo: CargoBuildCmd) -> Self { Self::from_cargo(&cargo) }
}

/// System param for executing command actions
#[derive(SystemParam)]
pub struct CommandParams<'w, 's> {
	bevy_commands: Commands<'w, 's>,
	pkg_config: Res<'w, PackageConfig>,
}

impl CommandParams<'_, '_> {
	/// Run the provided command asynchronously,
	/// calling [`Outcome::Pass`] if the command was successful or [`Outcome::Fail`] if it failed.
	// /// All stdout and stderr lines are emitted as [`StdOutLine`] entity events.
	// ///
	// /// Lines are streamed concurrently as they arrive. Empty lines are emitted (not skipped).
	// /// Reader tasks are aborted if the process exits first.
	pub fn execute(
		&mut self,
		mut ev: On<GetOutcome>,
		cmd_config: impl Into<CommandConfig>,
	) -> Result {
		let CommandConfig {
			cmd,
			args,
			current_dir,
			envs,
			wait,
			kill,
		} = cmd_config.into();

		let envs = envs.clone().xtend(self.pkg_config.envs());
		if kill {
			self.bevy_commands
				.entity(ev.action())
				.remove::<ChildHandle>();
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
	}
}
/// An untyped command, for an example of a more
/// user-friendly command see [`CargoCommand`]
#[construct]
pub fn RawCommand(config: CommandConfig) -> impl Bundle {
	OnSpawn::observe(
		move |ev: On<GetOutcome>, mut cmd_params: CommandParams| {
			cmd_params.execute(ev, config.clone())
		},
	)
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, CliPlugin))
			.insert_resource(pkg_config!())
			.world_mut()
			.spawn((Sequence, ExitOnEnd, children![RawCommand {
				config: CommandConfig::parse("echo foobar")
			}]))
			.trigger_target(GetOutcome);

		app.run_async().await.xpect_eq(AppExit::Success);
	}
}
