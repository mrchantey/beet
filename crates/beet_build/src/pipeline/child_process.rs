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
			let args = args.clone();
			let envs = envs.clone().xtend(config.envs());
			commands.entity(ev.action()).remove::<ChildHandle>();

			ev.run_async(async move |mut action| {
				let envs_pretty = envs
					.iter()
					.map(|(k, v)| format!("{}={}", k, v))
					.collect::<Vec<_>>()
					.join(" ");
				info!("{} {} {}", envs_pretty, cmd, args.join(" "));
				// 1. spawn the command
				let mut child = async_process::Command::new(&cmd)
					.args(&args)
					.envs(envs)
					.spawn()?;
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
					match child.status().await?.exit_ok() {
						Ok(_) => Outcome::Pass,
						Err(_) => Outcome::Fail,
					}
				} else {
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
			wait: true,
			kill: true,
		}
	}
}

impl ChildProcess {
	pub fn from_cargo(cargo: &CargoBuildCmd) -> Self {
		Self {
			cmd: "cargo".into(),
			args: cargo.get_args().iter().map(|s| s.to_string()).collect(),
			envs: Vec::default(),
			wait: true,
			kill: true,
		}
	}
}


impl ChildProcess {
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
