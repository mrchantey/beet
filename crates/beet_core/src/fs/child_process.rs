use crate::prelude::*;
use std::io::ErrorKind;
use std::process::Output;

/// Helper for spawning processes with
/// easy stdout collection
#[derive(Debug, SetWith)]
pub struct ChildProcess<'a> {
	/// The command to run (e.g. "ls", "cargo")
	command: &'a str,
	/// Arguments to pass to the command
	args: &'a [&'a str],
	/// Optional working directory for the command. If `None`, uses the current directory.
	#[set_with(unwrap_option)]
	cwd: Option<&'a AbsPathBuf>,
	/// Optional error message to use if the command is not found. If `None`, uses the default error.
	#[set_with(unwrap_option)]
	not_found: Option<&'a str>,
}

impl std::fmt::Display for ChildProcess<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.command)?;
		for arg in self.args {
			write!(f, " {arg}")?;
		}
		Ok(())
	}
}

impl<'a> ChildProcess<'a> {
	/// Creates a new process with the given command and optional arguments.
	pub fn new(command: &'a str) -> Self {
		Self {
			command,
			args: &[],
			cwd: None,
			not_found: None,
		}
	}

	/// Run the command, collecting stdout
	#[track_caller]
	pub fn run(self) -> Result<Output> {
		let mut cmd = std::process::Command::new(self.command);
		if let Some(dir) = self.cwd {
			cmd.current_dir(dir);
		}
		cmd.args(self.args)
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
		let mut cmd = async_process::Command::new(self.command);
		if let Some(dir) = self.cwd {
			cmd.current_dir(dir);
		}
		cmd.args(self.args)
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
				&& let Some(msg) = self.not_found
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
				"process failed: {}\nexited with non-zero status: {}\n{}",
				self,
				output.status,
				String::from_utf8_lossy(&output.stderr)
			)
		}
	}
}
