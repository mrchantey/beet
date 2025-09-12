use crate::prelude::*;
use bevy::prelude::*;
use std::fmt::Display;
use std::process::Child;
use std::process::Command;


// TODO replace with beet_flow
#[derive(Reflect, Component)]
pub struct ChildProcessSequence {
	/// whether to kill all handles before each run,
	/// useful for when child2 needs the child1 process to end before it runs.
	kill_before_run: bool,
}

impl ChildProcessSequence {
	pub fn new(kill_before_run: bool) -> Self { Self { kill_before_run } }

	pub fn system(
		query: Query<(Entity, &ChildProcessSequence)>,
		children: Query<&Children>,
		mut processes: Query<&mut ChildProcess>,
	) -> Result {
		for (entity, sequence) in query.iter() {
			if sequence.kill_before_run {
				for child in children.iter_descendants_depth_first(entity) {
					if let Ok(mut child) = processes.get_mut(child) {
						child.try_kill()?;
					}
				}
			}
			for child in children.iter_descendants_depth_first(entity) {
				if let Ok(mut child) = processes.get_mut(child) {
					child.run()?;
				}
			}
		}
		Ok(())
	}
}

#[derive(Reflect, Component)]
pub struct ChildProcess {
	/// Arguments from which the [`Command`] will be created, ie `vec!["cargo","run"]`
	pub args: Vec<String>,
	pub run_mode: ExecProcessMode,
	#[reflect(ignore)]
	/// If run with [`ExecProcessMode::SpawnHandle`], the handle will be stored here
	pub handle: Option<Child>,
	/// When set errors will be printed but will not terminate the program
	pub ignore_errors: bool,
}

#[derive(Clone, Reflect)]
pub enum ExecProcessMode {
	/// Block the app on this command running, ie a compile step
	Blocking,
	/// Spawn and hold onto a handle, ie spinning up a server
	SpawnHandle,
}


impl ChildProcess {
	pub fn new<S: AsRef<str>>(
		args: Vec<S>,
		run_mode: ExecProcessMode,
		ignore_errors: bool,
	) -> Self {
		Self {
			args: args.into_iter().map(|s| s.as_ref().to_string()).collect(),
			run_mode,
			handle: None,
			ignore_errors,
		}
	}

	pub fn try_kill(&mut self) -> Result<()> {
		if let Some(mut handle) = self.handle.take() {
			debug!("Killing Child Process: {}", self.args.join(" "));
			handle.kill().xmap(|res| self.handle_error(res))?;
		}
		Ok(())
	}

	/// Run the command in its [`ExecProcessMode`]
	pub fn run(&mut self) -> Result {
		self.try_kill()?;

		debug!("Running Child Process: {}", self.args.join(" "));
		if self.args.is_empty() {
			return self
				.handle_error(Err(BevyhowError::new("command is empty")));
		}
		let mut cmd = Command::new(&self.args[0]);
		cmd.args(&self.args[1..]);

		match self.run_mode {
			ExecProcessMode::Blocking => {
				cmd.status()
					.xmap(|res| self.handle_error(res))?
					.exit_ok()
					.xmap(|res| self.handle_error(res))?;
			}
			ExecProcessMode::SpawnHandle => {
				self.handle = cmd
					.spawn()
					.map(|res| Some(res))
					// returns None if err and Self::ignore_errors
					.xmap(|res| self.handle_error(res))?
			}
		};

		Ok(())
	}

	fn handle_error<
		T: Default,
		E: 'static + Send + Sync + Display + Into<BevyError>,
	>(
		&self,
		result: Result<T, E>,
	) -> Result<T> {
		match (result, self.ignore_errors) {
			(Err(err), false) => Err(err.into()),
			(Err(err), true) => {
				error!("Child Process Error: {}", err);
				Ok(T::default())
			}
			(Ok(val), _) => Ok(val),
		}
	}
}



#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	// use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();
		world.spawn((ChildProcessSequence::new(false), children![
			// no commands
			ChildProcess::new::<String>(
				vec![],
				ExecProcessMode::Blocking,
				true
			),
			// bad command
			ChildProcess::new(
				vec!["this_command_should_not_exist_12345"],
				ExecProcessMode::Blocking,
				true,
			),
			// one command
			ChildProcess::new(vec!["echo"], ExecProcessMode::Blocking, false),
			// sequence 1
			ChildProcess::new(
				vec!["echo", "howdy"],
				ExecProcessMode::Blocking,
				false,
			),
			// sequence 2
			children![ChildProcess::new(
				vec!["echo", "doody"],
				ExecProcessMode::Blocking,
				false,
			)],
		]));
		world
			.run_system_cached(ChildProcessSequence::system)
			.unwrap()
			.unwrap();
	}

	#[test]
	#[should_panic]
	fn panics_on_invalid_command() {
		let mut world = World::new();
		world.spawn((ChildProcessSequence::new(false), children![
			ChildProcess::new(
				vec!["this_command_should_not_exist_12345"],
				ExecProcessMode::Blocking,
				false,
			),
		]));
		world
			.run_system_cached(ChildProcessSequence::system)
			.unwrap()
			.unwrap();
	}
}
