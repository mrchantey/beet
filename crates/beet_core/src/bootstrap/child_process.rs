//! The description of a child process launch, and (on native `fs` builds) the
//! machinery that runs it.
//!
//! The DESCRIPTION compiles everywhere: a wasm consumer authors a
//! [`ChildProcess`] (ie a [`BuildArtifact`]'s build command) it cannot itself
//! run, exactly as it authors a stack it cannot apply. Everything that touches
//! `std::process` rides the `fs` feature and native targets.
use crate::prelude::*;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
use std::io::ErrorKind;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
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
	/// Values that must never be printed, see [`with_secret`](Self::with_secret).
	#[set_with(skip)]
	secrets: Vec<SmolStr>,
}

impl std::fmt::Display for ChildProcess {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.redact(&self.command))?;
		for arg in &self.args {
			write!(f, " {}", self.redact(arg))?;
		}
		Ok(())
	}
}

/// Handle for a long-running child process.
/// Kills the process on drop, and also supports explicit [`kill`](ChildHandle::kill).
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub struct ChildHandle {
	inner: async_process::Child,
	/// The child leads its own process group (see [`ChildProcess::with_group`]),
	/// so kill targets the group, not just the immediate child.
	group: bool,
}

#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
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

	/// Take the child's piped stdin, `None` unless spawned with
	/// [`spawn_piped`](ChildProcess::spawn_piped) or already taken.
	pub fn take_stdin(&mut self) -> Option<async_process::ChildStdin> {
		self.inner.stdin.take()
	}

	/// Take the child's piped stdout, `None` unless spawned with
	/// [`spawn_piped`](ChildProcess::spawn_piped) or already taken.
	pub fn take_stdout(&mut self) -> Option<async_process::ChildStdout> {
		self.inner.stdout.take()
	}

	/// Take the child's piped stderr, `None` unless spawned with
	/// [`spawn_piped`](ChildProcess::spawn_piped) or already taken.
	pub fn take_stderr(&mut self) -> Option<async_process::ChildStderr> {
		self.inner.stderr.take()
	}
}

#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
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
			secrets: Vec::new(),
		}
	}

	/// Declare a value this invocation carries that must never be printed.
	///
	/// A failed command reports its own argv (which is how a reader knows what
	/// failed), so a password passed as an argument would otherwise land in the
	/// deploy log, the terminal scrollback and whatever ingests either. Every
	/// occurrence is replaced in the command's [`Display`] and in the error a
	/// non-zero exit raises, including inside the child's own stderr, which is
	/// where a cli is most likely to echo an argument back.
	///
	/// This is about what gets WRITTEN DOWN. The argument is still in the
	/// process's argv, exactly as `tofu apply -var` has always been.
	pub fn with_secret(mut self, secret: impl Into<SmolStr>) -> Self {
		let secret = secret.into();
		if !secret.is_empty() {
			self.secrets.push(secret);
		}
		self
	}

	/// `text` with every declared secret replaced.
	fn redact(&self, text: &str) -> String {
		self.secrets.iter().fold(text.to_string(), |text, secret| {
			text.replace(secret.as_str(), Self::REDACTED)
		})
	}

	/// What a declared secret prints as.
	pub const REDACTED: &'static str = "<redacted>";

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

	/// Deliver `config` to a beet child process: scrub every inherited `BEET_*`
	/// var from its environment, then append the config's argv.
	///
	/// When beet spawns beet, ambient config inheritance is a bug class rather
	/// than a convenience: the parent constructs the child's config field by
	/// field and this hands it over, so a leak is unrepresentable and propagation
	/// is code you can read.
	///
	/// ```ignore
	/// let child = BootstrapConfig {
	///     repo: config.repo.clone(), // deliberate inheritance
	///     ..default()                  // everything else: not inherited
	/// };
	/// ChildProcess::new("beet").with_bootstrap(&child)?
	/// ```
	///
	/// Only `BEET_*` names are scrubbed, so SDK-convention secrets
	/// (`AWS_ACCESS_KEY_ID`, …) still inherit where a child legitimately needs
	/// them.
	pub fn with_bootstrap(mut self, config: &BootstrapConfig) -> Result<Self> {
		self.env_removals.extend(
			env_ext::vars()
				.into_iter()
				.map(|(key, _)| key)
				.filter(|key| key.starts_with("BEET_"))
				.map(SmolStr::from),
		);
		self.args.extend(config.to_argv()?);
		self.xok()
	}
}

/// Running the described process: native-only, since the description alone is
/// what a wasm consumer holds.
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
impl ChildProcess {
	/// The configured command: program, args, cwd, env additions and removals,
	/// and the unix process group when requested. The single place that
	/// translation happens, so every run/spawn variant below is only a choice of
	/// how to drive it.
	///
	/// Built as a `std` command because the unix process-group extension applies
	/// there and `async_process`'s sealed `CommandExt` does not expose it; an
	/// async caller converts with `async_process::Command::from`.
	fn into_command_std(&self) -> std::process::Command {
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
		cmd.args(self.args.iter().map(SmolStr::as_str));
		#[cfg(unix)]
		if self.group {
			use std::os::unix::process::CommandExt;
			// pgid 0 = a fresh group led by the child, so kill can target `-pid`.
			cmd.process_group(0);
		}
		cmd
	}

	/// Run the command, collecting stdout
	#[track_caller]
	pub fn run(self) -> Result<Output> {
		self.into_command_std()
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

	/// Convert this `ChildProcess` into an `async_process::Command` without
	/// running it.
	pub fn into_command_async(self) -> async_process::Command {
		self.into_command_std().into()
	}

	/// Run the command asynchronously using `async_process`, collecting stdout.
	pub async fn run_async(self) -> Result<Output> {
		async_process::Command::from(self.into_command_std())
			.output()
			.await
			.xmap(|result| self.map_result(result))?
			.xmap(|output| self.map_output(output))
	}

	/// Run the command asynchronously with `input` written to its stdin, which
	/// is then closed, collecting stdout.
	///
	/// The shape a filter takes: a value handed to a tool and its answer read
	/// back, with nothing landing on disk in between. That matters when the
	/// value is a private key, which is why this exists rather than a temporary
	/// file and two invocations.
	pub async fn run_async_stdin(
		self,
		input: impl AsRef<[u8]>,
	) -> Result<Output> {
		use futures_lite::AsyncWriteExt;
		let mut cmd = async_process::Command::from(self.into_command_std());
		cmd.stdin(std::process::Stdio::piped())
			.stdout(std::process::Stdio::piped())
			.stderr(std::process::Stdio::piped());
		let mut child = cmd.spawn().map_err(|err| self.map_spawn_error(err))?;
		let mut stdin = child
			.stdin
			.take()
			.ok_or_else(|| bevyhow!("{self}: stdin was not piped"))?;
		stdin.write_all(input.as_ref()).await?;
		// the child reads to EOF, so the pipe must close before the wait below
		drop(stdin);
		child
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

	/// Spawn the command as a long-running child process, sharing the parent's
	/// stdio. Returns a [`ChildHandle`] that kills the process on drop.
	pub fn spawn(self) -> Result<ChildHandle> {
		self.spawn_with(async_process::Command::from(self.into_command_std()))
	}

	/// Spawn the command with stdin, stdout and stderr piped, so the caller
	/// writes the child's input and reads its output instead of the child
	/// sharing the terminal.
	///
	/// The shape a protocol-speaking child needs: a request written to
	/// [`take_stdin`](ChildHandle::take_stdin), an event stream read from
	/// [`take_stdout`](ChildHandle::take_stdout).
	pub fn spawn_piped(self) -> Result<ChildHandle> {
		let mut cmd = async_process::Command::from(self.into_command_std());
		cmd.stdin(std::process::Stdio::piped())
			.stdout(std::process::Stdio::piped())
			.stderr(std::process::Stdio::piped());
		self.spawn_with(cmd)
	}

	/// A spawn failure, with a missing executable mapped onto the configured
	/// [`not_found`](Self::with_not_found) message.
	fn map_spawn_error(&self, err: std::io::Error) -> BevyError {
		match err.kind() == ErrorKind::NotFound {
			true => match &self.not_found {
				Some(msg) => bevyhow!("{msg}"),
				None => err.into(),
			},
			false => err.into(),
		}
	}

	/// Spawn a prepared command, mapping a missing executable onto the
	/// configured [`not_found`](Self::with_not_found) message.
	fn spawn_with(
		&self,
		mut cmd: async_process::Command,
	) -> Result<ChildHandle> {
		let child = cmd.spawn().map_err(|err| self.map_spawn_error(err))?;
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
				self.redact(&String::from_utf8_lossy(&output.stderr))
			)
		}
	}
}

#[cfg(all(test, feature = "fs", not(target_arch = "wasm32")))]
mod test {
	use crate::prelude::*;

	/// A failed process reports its own argv, which is how a reader knows what
	/// failed and also how a password reaches a deploy log. A declared secret
	/// is gone from every rendering of the command.
	#[crate::test]
	fn a_declared_secret_never_prints() {
		ChildProcess::new("aws")
			.with_args(["ssm", "put-parameter", "--value", "hunter2"])
			.with_secret("hunter2")
			.to_string()
			.xpect_eq("aws ssm put-parameter --value <redacted>");
	}

	/// An undeclared value prints, which is the whole reason declaring is
	/// explicit: this type cannot guess which argument is the password.
	#[crate::test]
	fn an_undeclared_value_prints() {
		ChildProcess::new("echo")
			.with_args(["hunter2"])
			.to_string()
			.xpect_eq("echo hunter2");
	}

	/// The child's own stderr is where a cli is most likely to echo an argument
	/// back, so the redaction has to reach the error text too.
	#[crate::test]
	async fn a_secret_echoed_back_is_redacted() {
		let err = ChildProcess::new("sh")
			.with_args(["-c", "echo hunter2 >&2; exit 1"])
			.with_secret("hunter2")
			.run_async()
			.await
			.unwrap_err()
			.to_string();
		err.contains("hunter2").xpect_false();
		err.xpect_contains("<redacted>");
	}
}
