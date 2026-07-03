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
	/// Environment variables to remove from the inherited environment, eg an empty
	/// `AWS_PROFILE` the `aws` cli rejects.
	#[set_with(skip)]
	env_removals: Vec<SmolStr>,
	/// Optional working directory for the command. If `None`, uses the current directory.
	#[set_with(unwrap_option)]
	cwd: Option<AbsPathBuf>,
	/// Optional error message to use if the command is not found. If `None`, uses the default error.
	#[set_with(unwrap_option)]
	not_found: Option<SmolStr>,
	/// Spawn the child into its own process group (unix only), so
	/// [`ChildHandle::kill`] takes down the whole tree — a cli that is really a
	/// wrapper script (eg `wrangler`) otherwise leaves its real process running
	/// after the wrapper dies, holding inherited stdio open. Opt-in because a
	/// grouped child no longer receives the terminal's Ctrl+C with the parent;
	/// right for a child the caller kills itself (eg a bounded log tail), wrong
	/// for an interactive child the user stops (eg a monitor).
	group: bool,
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

/// Handle for a long-running child process.
/// Kills the process on drop, and also supports explicit [`kill`](ChildHandle::kill).
pub struct ChildHandle {
	inner: async_process::Child,
	/// The child leads its own process group (see [`ChildProcess::with_group`]),
	/// so kill targets the group, not just the immediate child.
	group: bool,
}

impl ChildHandle {
	/// Kill the child process — the whole process group for a
	/// [`with_group`](ChildProcess::with_group) child, so a wrapper-script cli's
	/// real process dies with it.
	pub fn kill(&mut self) -> Result<()> {
		#[cfg(unix)]
		if self.group {
			// `process_group(0)` at spawn made the child the group leader, so its
			// pid is the pgid; `kill -9 -- -PGID` signals the whole group. Run via
			// bash's *builtin* kill: the standalone /usr/bin/kill (util-linux)
			// rejects the negative-pgid form that the builtin accepts. Shelling out
			// keeps this libc-free (this is process-spawning code anyway).
			std::process::Command::new("bash")
				.args(["-c", &format!("kill -9 -- -{}", self.inner.id())])
				.output()
				.ok();
		}
		self.inner
			.kill()
			.map_err(|err| bevyhow!("failed to kill child process: {err}"))
	}

	/// Wait for the child process to complete and return its exit status.
	pub async fn status(&mut self) -> Result<std::process::ExitStatus> {
		self.inner
			.status()
			.await
			.map_err(|err| bevyhow!("child process failed: {err}"))
	}
}

impl Drop for ChildHandle {
	fn drop(&mut self) { self.kill().ok(); }
}

impl ChildProcess {
	/// Creates a new process with the given command and optional arguments.
	pub fn new(command: impl Into<SmolStr>) -> Self {
		Self {
			command: command.into(),
			args: Vec::new(),
			envs: Vec::new(),
			env_removals: Vec::new(),
			cwd: None,
			not_found: None,
			group: false,
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
		self.envs = envs
			.into_iter()
			.map(|(k, v)| (k.into(), v.into()))
			.collect();
		self
	}

	/// Remove an environment variable from the inherited environment for the child
	/// process. Needed when an inherited var is actively harmful, eg an empty
	/// `AWS_PROFILE` (`AWS_PROFILE=`) which the `aws` cli reads as a profile literally
	/// named `""` and rejects, rather than falling back to explicit keys.
	pub fn without_env(mut self, key: impl Into<SmolStr>) -> Self {
		self.env_removals.push(key.into());
		self
	}

	/// Run the command, collecting stdout
	#[track_caller]
	pub fn run(self) -> Result<Output> {
		let mut cmd = std::process::Command::new(self.command.as_str());
		for (key, val) in &self.envs {
			cmd.env(key.as_str(), val.as_str());
		}
		for key in &self.env_removals {
			cmd.env_remove(key.as_str());
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

	/// Convert this `ChildProcess` into a `std::process::Command` without running it.
	pub fn into_command_async(self) -> async_process::Command {
		let mut cmd = async_process::Command::new(self.command.as_str());
		for (key, val) in &self.envs {
			cmd.env(key.as_str(), val.as_str());
		}
		for key in &self.env_removals {
			cmd.env_remove(key.as_str());
		}
		if let Some(dir) = &self.cwd {
			cmd.current_dir(dir);
		}
		cmd.args(self.args.iter().map(SmolStr::as_str));
		cmd
	}

	/// Run the command asynchronously using `async_process`, collecting stdout.
	pub async fn run_async(self) -> Result<Output> {
		let mut cmd = async_process::Command::new(self.command.as_str());
		for (key, val) in &self.envs {
			cmd.env(key.as_str(), val.as_str());
		}
		for key in &self.env_removals {
			cmd.env_remove(key.as_str());
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

	/// Spawn the command as a long-running child process.
	/// Returns a [`ChildHandle`] that kills the process on drop.
	pub fn spawn(self) -> Result<ChildHandle> {
		// built as a std Command so the unix process-group extension applies
		// (async_process's sealed CommandExt does not expose it), then converted.
		let mut std_cmd = std::process::Command::new(self.command.as_str());
		for (key, val) in &self.envs {
			std_cmd.env(key.as_str(), val.as_str());
		}
		for key in &self.env_removals {
			std_cmd.env_remove(key.as_str());
		}
		if let Some(dir) = &self.cwd {
			std_cmd.current_dir(dir);
		}
		std_cmd.args(self.args.iter().map(SmolStr::as_str));
		#[cfg(unix)]
		if self.group {
			use std::os::unix::process::CommandExt;
			// pgid 0 = a fresh group led by the child, so kill can target `-pid`.
			std_cmd.process_group(0);
		}
		let child = async_process::Command::from(std_cmd).spawn().map_err(
			|err| {
				if err.kind() == ErrorKind::NotFound
					&& let Some(msg) = &self.not_found
				{
					bevyhow!("{msg}")
				} else {
					err.into()
				}
			},
		)?;
		Ok(ChildHandle {
			inner: child,
			group: self.group,
		})
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
