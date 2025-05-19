use anyhow::Result;
use std::process::Child;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;


pub struct CommandExt;

impl CommandExt {
	/// Create a command from a vector
	pub fn from_vec<T: AsRef<str>>(cmd: &Vec<T>) -> Command {
		let mut command = Command::new(cmd[0].as_ref());
		for arg in cmd.iter().skip(1) {
			command.arg(arg.as_ref());
		}
		command
	}


	/// Create a command from a string, splitting on whitespace
	pub fn from_whitespace(cmd: &str) -> Command {
		let cmd = cmd.split_whitespace();
		let mut command = Command::new(cmd.clone().next().unwrap());
		for arg in cmd.skip(1) {
			command.arg(arg);
		}
		command
	}

	/// Turn an exit status into a Result
	pub fn unwrap_status(mut cmd: Command) -> Result<()> {
		let status = cmd.status()?;
		if !status.success() {
			Err(anyhow::anyhow!("Command failed: {:?}", status))?;
		}
		Ok(())
	}
	/// Turn a non-empty output into a Result
	pub fn unwrap_output_empty(mut cmd: Command) -> Result<()> {
		let output = cmd.output()?;
		if output.stdout.is_empty() && output.stderr.is_empty() {
			Ok(())
		} else {
			anyhow::bail!(
				"Expected empty output, received: \nStdout: {}\nStderr: {}",
				String::from_utf8_lossy(&output.stdout),
				String::from_utf8_lossy(&output.stderr)
			)
		}
	}


	/// Run a command and pipe the output to stdio. Returns error only if execution failed, not if it returns error
	pub fn spawn_command(args: &Vec<&str>) -> Result<Child> {
		println!("{}", args.join(" "));
		let child = Self::get_command(args)
			// .stdout(Stdio::piped())
			.spawn()?;
		Ok(child)
	}

	pub fn spawn_command_blocking(args: &Vec<&str>) -> Result<()> {
		let _ = Self::get_command(args)
			.stdout(Stdio::inherit())
			.stderr(Stdio::inherit())
			.output()?;
		Ok(())
	}

	pub fn spawn_command_hold_stdio(args: &Vec<&str>) -> Result<CommandOutput> {
		let out = Self::get_command(args).output()?;
		Ok(out.into())
	}

	fn get_command(args: &Vec<&str>) -> Command {
		let mut cmd = Command::new(args[0]);
		cmd.args(args[1..].iter());
		cmd
	}

	pub fn spawn_command_with_shell_blocking(args: &Vec<&str>) -> Result<()> {
		let _ = Self::get_command_with_shell(args)
			.stdout(Stdio::inherit())
			.stderr(Stdio::inherit())
			.output()?;
		Ok(())
	}

	fn get_command_with_shell(args: &Vec<&str>) -> Command {
		let is_windows = cfg!(target_os = "windows");
		let (cmd, arg) = if is_windows {
			// ("cmd", "\\C")
			("powershell", "-Command")
		} else {
			("sh", "-c")
		};
		let mut cmd = Command::new(cmd);
		cmd.arg(arg);
		cmd.args(args);
		cmd
	}
}

pub struct CommandOutput {
	pub success: bool,
	pub stdout: String,
	pub stderr: String,
}

impl From<Output> for CommandOutput {
	fn from(output: Output) -> Self {
		let stdout = String::from_utf8_lossy(&output.stdout).to_string();
		let stderr = String::from_utf8_lossy(&output.stderr).to_string();
		CommandOutput {
			success: output.status.success(),
			stdout,
			stderr,
		}
	}
}
