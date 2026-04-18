use crate::prelude::*;
use std::io::ErrorKind;
use std::process::Output;

/// Helper for spawning processes with
/// easy stdout collection
#[derive(Debug, Clone, SetWith)]
pub struct ChildProcess {
	/// The command to run (e.g. "ls", "cargo")
	command: SmolStr,
	/// Arguments to pass to the command
	#[set_with(skip)]
	args: Vec<SmolStr>,
	/// Environment variables to set for the child process.
	#[set_with(skip)]
	envs: Vec<(SmolStr, SmolStr)>,
	/// Optional working directory for the command. If `None`, uses the current directory.
	#[set_with(unwrap_option)]
	cwd: Option<AbsPathBuf>,
	/// Optional error message to use if the command is not found. If `None`, uses the default error.
	#[set_with(unwrap_option)]
	not_found: Option<SmolStr>,
}

impl std::fmt::Display for ChildProcess {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.command)?;
		for arg in &self.args {
			write!(f, " {arg}")?;
		}
		Ok(())
	}
}

impl ChildProcess {
	/// Creates a new process with the given command and optional arguments.
	pub fn new(command: impl Into<SmolStr>) -> Self {
		Self {
			command: command.into(),
			args: Vec::new(),
			envs: Vec::new(),
			cwd: None,
			not_found: None,
		}
	}

	/// Sets the arguments to pass to the command.
	pub fn with_args(
		mut self,
		args: impl IntoIterator<Item = impl Into<SmolStr>>,
	) -> Self {
		self.args = args.into_iter().map(Into::into).collect();
		self
	}

	/// Sets environment variables for the child process.
	pub fn with_envs(
		mut self,
		envs: impl IntoIterator<Item = (impl Into<SmolStr>, impl Into<SmolStr>)>,
	) -> Self {
		self.envs = envs.into_iter().map(|(k, v)| (k.into(), v.into())).collect();
		self
	}

	/// Run the command, collecting stdout
	#[track_caller]
	pub fn run(self) -> Result<Output> {
		let mut cmd = std::process::Command::new(self.command.as_str());
		for (key, val) in &self.envs {
			cmd.env(key.as_str(), val.as_str());
		}
		if let Some(dir) = &self.cwd {
			cmd.current_dir(dir);
		}
		cmd.args(self.args.iter().map(SmolStr::as_str))
			.output()
			.xmap(|result| self.map_result(result))?
			.xmap(|output| self.map_output(output))
	}

	/// Run the command, collecting stdout
	#[track_caller]
	pub fn run_stdout(self) -> Result<String> {
		self.run()
			.map(|output| String::from_utf8_lossy(&output.stdout).to_string())
	}
	/// Run the command asynchronously using `async_process`, collecting stdout.
	pub async fn run_async(self) -> Result<Output> {
		let mut cmd = async_process::Command::new(self.command.as_str());
		for (key, val) in &self.envs {
			cmd.env(key.as_str(), val.as_str());
		}
		if let Some(dir) = &self.cwd {
			cmd.current_dir(dir);
		}
		cmd.args(self.args.iter().map(SmolStr::as_str))
			.output()
			.await
			.xmap(|result| self.map_result(result))?
			.xmap(|output| self.map_output(output))
	}

	/// Run the command, collecting stdout
	pub async fn run_async_stdout(self) -> Result<String> {
		self.run_async()
			.await
			.map(|output| String::from_utf8_lossy(&output.stdout).to_string())
	}

	fn map_result(
		&self,
		result: Result<Output, std::io::Error>,
	) -> Result<Output> {
		result.map_err(|e| {
			if e.kind() == ErrorKind::NotFound
				&& let Some(msg) = &self.not_found
			{
				bevyhow!("{msg}")
			} else {
				e.into()
			}
		})
	}
	#[track_caller]
	fn map_output(&self, output: Output) -> Result<Output> {
		if output.status.success() {
			output.xok()
		} else {
			bevybail!(
				"process failed: {}
exited with non-zero status: {}
{}",
				self,
				output.status,
				String::from_utf8_lossy(&output.stderr)
			)
		}
	}
}
