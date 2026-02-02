use beet_core::prelude::*;
use beet_flow::prelude::*;
use std::path::PathBuf;

/// Emitted for each stdout/stderr line. `is_err` is true for stderr.
#[derive(EntityTargetEvent)]
pub struct StdOutLine {
	/// The text content of the line.
	pub line: String,
	/// True if this line should be routed to stderr, false for stdout.
	pub is_err: bool,
}

/// Holds a handle to a spawned child process, killed on drop.
/// This will only be inserted for a command executed on actions
/// with a [`ContinueRun`]. This component will be removed either
/// when the command is executed again or the action is cancelled.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ChildHandle(async_process::Child);

impl Drop for ChildHandle {
	fn drop(&mut self) { self.0.kill().ok(); }
}

/// Polls all running child processes for completion,
/// triggering the appropriate outcome and removing
/// the [`ChildHandle`] component when done.
pub fn poll_child_handles(
	mut commands: Commands,
	mut query: Populated<(Entity, &mut ChildHandle), With<Running>>,
) -> Result {
	for (action, mut child_handle) in query.iter_mut() {
		// try_status errors are an io::Error, we do not handle
		// and instead propagate
		if let Some(status) = child_handle.0.try_status()? {
			let outcome = match status.success() {
				true => Outcome::Pass,
				false => Outcome::Fail,
			};
			commands
				.entity(action)
				.remove::<ChildHandle>()
				.trigger_target(outcome);
		}
	}
	Ok(())
}


/// Removes any existing [`ChildHandle`] from an action
/// when the [`Running`] component is removed, interrupting
/// the process.
pub fn interrupt_child_handles(
	ev: On<Outcome>,
	mut commands: Commands,
	query: Query<(), With<ChildHandle>>,
	children: Query<&Children>,
) {
	for child in children.iter_descendants_inclusive(ev.target()) {
		if query.contains(child) {
			commands.entity(child).remove::<ChildHandle>();
		}
	}
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

	/// Accepts a full shell command string, and runs
	/// it via `sh -c <full_cmd>`
	pub fn parse_shell(full_cmd: impl Into<String>) -> Self {
		Self::from_parts("sh", vec!["-c".into(), full_cmd.into()])
	}


	/// Creates a command configuration from a command and arguments.
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

	/// Adds a single argument to the command.
	pub fn arg(mut self, arg: impl AsRef<str>) -> Self {
		self.args.push(arg.as_ref().to_string());
		self
	}

	/// Adds multiple arguments to the command.
	pub fn args(
		mut self,
		args: impl IntoIterator<Item = impl AsRef<str>>,
	) -> Self {
		self.args
			.extend(args.into_iter().map(|s| s.as_ref().to_string()));
		self
	}

	/// Creates a new command configuration with the given command.
	pub fn new(cmd: impl Into<String>) -> Self {
		Self {
			cmd: cmd.into(),
			..default()
		}
	}

	/// Sets the command to run.
	pub fn command(mut self, cmd: impl Into<String>) -> Self {
		self.cmd = cmd.into();
		self
	}

	/// Sets the current working directory for the command.
	pub fn current_dir(mut self, dir: impl Into<PathBuf>) -> Self {
		self.current_dir = Some(dir.into());
		self
	}

	/// Adds an environment variable to the command.
	pub fn env(
		mut self,
		key: impl Into<String>,
		value: impl Into<String>,
	) -> Self {
		self.envs.push((key.into(), value.into()));
		self
	}

	/// Creates a command configuration from a [`CargoBuildCmd`].
	pub fn from_cargo(cargo: &CargoBuildCmd) -> Self {
		Self {
			cmd: "cargo".into(),
			args: cargo.get_args().iter().map(|s| s.to_string()).collect(),
			..default()
		}
	}
	/// Converts this configuration into an action bundle.
	pub fn into_action(self) -> impl Bundle {
		OnSpawn::observe(
			move |ev: On<GetOutcome>, mut cmd_runner: CommandRunner| {
				cmd_runner.run(ev, self.clone())
			},
		)
	}
}

impl Default for CommandConfig {
	fn default() -> Self {
		Self {
			cmd: "true".into(),
			args: default(),
			current_dir: None,
			envs: default(),
		}
	}
}

impl From<CargoBuildCmd> for CommandConfig {
	fn from(cargo: CargoBuildCmd) -> Self { Self::from_cargo(&cargo) }
}

/// System param for executing command actions
#[derive(SystemParam)]
pub struct CommandRunner<'w, 's> {
	bevy_commands: Commands<'w, 's>,
	pkg_config: Res<'w, PackageConfig>,
	/// Used to check whether an action is interruptable
	interruptable: Query<'w, 's, &'static ContinueRun>,
	async_commands: AsyncCommands<'w, 's>,
}

impl CommandRunner<'_, '_> {
	/// Run the provided command asynchronously,
	/// calling [`Outcome::Pass`] if the command was successful or [`Outcome::Fail`] if it failed.
	// /// All stdout and stderr lines are emitted as [`StdOutLine`] entity events.
	// ///
	// /// Lines are streamed concurrently as they arrive. Empty lines are emitted (not skipped).
	// /// Reader tasks are aborted if the process exits first.
	pub fn run(
		&mut self,
		ev: On<GetOutcome>,
		cmd_config: impl Into<CommandConfig>,
	) -> Result {
		let CommandConfig {
			cmd,
			args,
			current_dir,
			envs,
		} = cmd_config.into();

		let action = ev.target();
		let interruptable = self.interruptable.contains(action);

		let envs = envs.clone().xtend(self.pkg_config.envs());
		self.bevy_commands.entity(action).remove::<ChildHandle>();
		let envs_pretty = envs
			.iter()
			.map(|(k, v)| format!("{}={}", k, v))
			.collect::<Vec<_>>()
			.join(" ");
		info!("{} {} {}", envs_pretty, cmd, args.join(" "));
		// 1. spawn the command
		let mut cmd = async_process::Command::new(&cmd);
		cmd.args(&args).envs(envs).kill_on_drop(true);
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
		if interruptable {
			// store the child process and poll for completion
			self.bevy_commands
				.entity(action)
				// TODO this only allows a single child process per action
				.insert(ChildHandle(child));
		} else {
			self.async_commands.run(async move |world: AsyncWorld| {
				// wait for completion
				let outcome = match child.status().await {
					Ok(status) if status.success() => Outcome::Pass,
					_ => Outcome::Fail,
				};
				world
					.with_then(move |world| {
						world.entity_mut(action).trigger_target(outcome);
					})
					.await;
			});
		}
		Ok(())
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;

	#[beet_core::test]
	async fn works() {
		let mut app = App::new();
		app.add_plugins(CliPlugin)
			.world_mut()
			.spawn((Sequence, ExitOnEnd, children![
				CommandConfig::parse("true").into_action()
			]))
			.trigger_target(GetOutcome);

		app.run_async().await.xpect_eq(AppExit::Success);
	}
	#[beet_core::test]
	async fn continue_run_pass() {
		let mut app = App::new();
		app.add_plugins(CliPlugin)
			.world_mut()
			.spawn((Sequence, ExitOnEnd, children![(
				ContinueRun,
				CommandConfig::parse("true").into_action()
			)]))
			.trigger_target(GetOutcome);
		app.run_async().await.xpect_eq(AppExit::Success);
	}
	#[beet_core::test]
	async fn continue_run_fail() {
		let mut app = App::new();
		app.add_plugins(CliPlugin)
			.world_mut()
			.spawn((Sequence, ExitOnEnd, children![(
				ContinueRun,
				CommandConfig::parse("false").into_action()
			)]))
			.trigger_target(GetOutcome);
		app.run_async().await.xpect_eq(AppExit::from_code(1));
	}

	#[test]
	fn interrupt_static() {
		let mut app = App::new();
		let entity = app
			.add_plugins(CliPlugin)
			.world_mut()
			.spawn((Sequence, ExitOnFail, children![(
				ContinueRun,
				CommandConfig::parse("false").into_action()
			)]))
			.trigger_target(GetOutcome)
			.id();

		app.world_mut().flush();

		// 1. child handle was inserted
		app.world_mut()
			.query_once::<&ChildHandle>()
			.len()
			.xpect_eq(1);

		app.world_mut()
			.entity_mut(entity)
			.trigger_target(Outcome::Pass);

		app.world_mut().flush();

		// 2. child handle was removed
		app.world_mut()
			.query_once::<&ChildHandle>()
			.len()
			.xpect_eq(0);
	}
	#[beet_core::test]
	async fn interrupt_timed() {
		let mut app = App::new();
		let entity = app
			.add_plugins(CliPlugin)
			.world_mut()
			.spawn((Sequence, ExitOnFail, children![(
				ContinueRun,
				// sleep at least 10 millis
				CommandConfig::parse_shell("sleep 0.01 && false").into_action()
			)]))
			.trigger_target(GetOutcome)
			.id();

		app.world_mut().run_async(async move |world| {
			// short sleep
			time_ext::sleep_millis(2).await;
			// passing early interrupts child process
			// uncomment this line to fail the test
			world
				.entity(entity)
				.trigger_target_then(Outcome::Pass)
				.await;

			// wait to ensure process didnt fail
			time_ext::sleep_millis(20).await;
			world.write_message(AppExit::Success);
		});
		app.run_async().await.xpect_eq(AppExit::Success);
	}
}
