//! Run an arbitrary external process as a behaviour-tree leaf.
use crate::prelude::*;
use beet_core::prelude::*;

/// Runs an external process as a behaviour-tree step, streaming its stdout/stderr
/// live to the terminal and failing (propagating an error) on a non-zero exit.
///
/// A BSX-authorable leaf: `exe` is required, `args`/`cwd`/`env` are optional, so
/// a bare `<Command exe=".."/>` is enough to make the entity runnable.
///
/// Output streams live because the child inherits the parent's stdio (see
/// [`ChildProcess::spawn`]), which suits multi-minute builds where you want to
/// watch progress rather than wait for a buffered dump.
///
/// # Example
/// ```bsx
/// <Command exe="cargo" args={["build","--release"]} cwd="crates/foo" env={["RUST_LOG=info"]}/>
/// ```
///
/// ## Errors
/// - Errors if `exe` is empty.
/// - Errors if the working directory cannot be resolved to an absolute path.
/// - Errors if the process fails to spawn or exits non-zero.
#[action]
#[derive(Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn Command(
	/// The executable to run, eg `"cargo"` or `"sh"`. Required; an empty `exe`
	/// errors when the action runs.
	#[field]
	exe: SmolStr,
	/// Arguments passed to `exe`, authored as `args={["build","--release"]}`.
	#[field]
	args: Vec<SmolStr>,
	/// Working directory for the child. Empty inherits the current directory; a
	/// relative path resolves against the current directory, an absolute path is
	/// used as-is.
	#[field]
	cwd: SmolStr,
	/// Environment variables to set, each `"KEY=VALUE"`, authored as
	/// `env={["KEY=val"]}`. These are added to (not replacing) the inherited env.
	#[field]
	env: Vec<SmolStr>,
) -> Result<Outcome> {
	if exe.is_empty() {
		bevybail!("`Command` requires a non-empty `exe`");
	}

	// build the process, parsing each `KEY=VALUE` env entry (an entry without a
	// `=` sets the key to an empty value).
	let mut proc =
		ChildProcess::new(exe.clone()).with_args(args.iter().cloned());
	if !env.is_empty() {
		proc = proc.with_envs(env.iter().map(
			|entry| match entry.split_once('=') {
				Some((key, val)) => (key.to_string(), val.to_string()),
				None => (entry.to_string(), String::new()),
			},
		));
	}

	// resolve a non-empty cwd to an absolute path (relative paths resolve against
	// the current directory).
	if !cwd.is_empty() {
		let abs = if std::path::Path::new(cwd.as_str()).is_absolute() {
			AbsPathBuf::new(cwd.as_str())?
		} else {
			AbsPathBuf::new(std::env::current_dir()?.join(cwd.as_str()))?
		};
		proc = proc.with_cwd(abs);
	}

	// spawn (inherits parent stdio, so output streams live) and wait.
	info!("running: {proc}");
	let mut handle = proc.spawn()?;
	let status = handle.status().await?;
	if !status.success() {
		bevybail!("`{exe}` exited with {status}");
	}
	Outcome::PASS.xok()
}

impl Command {
	/// Create a command for the given executable, with no args/cwd/env.
	pub fn new(exe: impl Into<SmolStr>) -> Self {
		Self {
			exe: exe.into(),
			..default()
		}
	}

	/// Set the arguments passed to the executable.
	pub fn with_args(
		mut self,
		args: impl IntoIterator<Item = impl Into<SmolStr>>,
	) -> Self {
		self.args = args.into_iter().map(Into::into).collect();
		self
	}

	/// Set the environment variables, each `"KEY=VALUE"`.
	pub fn with_env(
		mut self,
		env: impl IntoIterator<Item = impl Into<SmolStr>>,
	) -> Self {
		self.env = env.into_iter().map(Into::into).collect();
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	async fn runs_and_passes() {
		AsyncPlugin::world()
			.spawn(Command::new("true"))
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}

	#[beet_core::test]
	async fn non_zero_exit_errors() {
		AsyncPlugin::world()
			.spawn(Command::new("false"))
			.call::<(), Outcome>(())
			.await
			.xpect_err();
	}

	#[beet_core::test]
	async fn empty_exe_errors() {
		AsyncPlugin::world()
			.spawn(Command::default())
			.call::<(), Outcome>(())
			.await
			.xpect_err();
	}

	#[beet_core::test]
	async fn args_and_env_resolve() {
		// `sh -c 'test "$FOO" = bar'` exits 0 only when the env arrived.
		AsyncPlugin::world()
			.spawn(
				Command::new("sh")
					.with_args(["-c", r#"test "$FOO" = bar"#])
					.with_env(["FOO=bar"]),
			)
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
}
