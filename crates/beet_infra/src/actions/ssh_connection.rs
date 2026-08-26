//! The management channel to a deployed box: the deploy reaches it to roll a
//! running instance onto a new release ([`LightsailRelease`]), and the same
//! helpers serve manual inspection of a box that is misbehaving.
use crate::prelude::*;
use beet_core::prelude::*;
use std::process::Output;

/// SSH connection details for a remote instance.
#[derive(Debug, Clone)]
pub struct SshConnection {
	/// The public IP or hostname of the instance.
	pub host: String,
	/// The SSH user, ie `ec2-user` or `ubuntu`.
	pub user: String,
	/// The port the sshd listens on. Not 22 whenever the deployed app serves
	/// its own ssh there, see [`LightsailBlock::management_ssh_port`].
	pub port: u16,
	/// Path to the private key file on disk.
	pub key_path: AbsPathBuf,
}

impl SshConnection {
	/// Standard SSH options for connecting to instances.
	/// Disables host key checking, uses a 30-second connection timeout, and
	/// keeps the session alive: a release can legitimately sit near-silent for
	/// minutes while a fresh box crawls through cloud-init, and the keepalive
	/// makes a dropped session fail fast instead of hanging the deploy.
	pub const OPTS: [&str; 10] = [
		"-o",
		"StrictHostKeyChecking=no",
		"-o",
		"UserKnownHostsFile=/dev/null",
		"-o",
		"ConnectTimeout=30",
		"-o",
		"BatchMode=yes",
		"-o",
		"ServerAliveInterval=15",
	];

	/// The `user@host` string for SSH commands.
	pub fn remote_user(&self) -> String {
		format!("{}@{}", self.user, self.host)
	}

	/// The shared arguments: the standard options and the identity file. The
	/// port is not among them, since `ssh` spells it `-p` and `scp` `-P`.
	fn ssh_args(&self) -> Vec<SmolStr> {
		let mut args: Vec<SmolStr> =
			Self::OPTS.iter().map(|opt| SmolStr::from(*opt)).collect();
		args.push("-i".into());
		args.push(self.key_path.display().to_string().into());
		args
	}

	/// Run a command on the remote instance via SSH, returning its output. A
	/// non-zero exit is an error carrying the remote stderr.
	pub async fn run_command(&self, command: &str) -> Result<Output> {
		let mut args = self.ssh_args();
		args.push("-p".into());
		args.push(self.port.to_string().into());
		args.push(self.remote_user().into());
		args.push(command.into());
		ChildProcess::new("ssh").with_args(args).run_async().await
	}

	/// Copy a local file to the remote instance via SCP.
	pub async fn scp_to(
		&self,
		local_path: &std::path::Path,
		remote_path: &str,
	) -> Result {
		let mut args = self.ssh_args();
		args.push("-P".into());
		args.push(self.port.to_string().into());
		args.push(local_path.display().to_string().into());
		args.push(format!("{}:{}", self.remote_user(), remote_path).into());
		ChildProcess::new("scp").with_args(args).run_async().await?;
		Ok(())
	}

	/// Wait for SSH to become available, retrying every `poll` until `timeout`
	/// elapses. At least one attempt is always made.
	pub async fn wait_for_ready(
		&self,
		timeout: Duration,
		poll: Duration,
	) -> Result {
		let max_attempts = (timeout.as_secs() / poll.as_secs().max(1)).max(1);
		for attempt in 1..=max_attempts {
			info!(
				"waiting for ssh on {}:{} (attempt {attempt}/{max_attempts})...",
				self.host, self.port
			);
			if self.run_command("echo ready").await.is_ok() {
				return Ok(());
			}
			time_ext::sleep(poll).await;
		}
		bevybail!(
			"failed to connect to {}:{} after {max_attempts} attempts",
			self.host,
			self.port
		)
	}
}

impl SshConnection {
	/// Read the box's ssh details from a deployed project's tofu outputs,
	/// caching the key pair's private key at `{work_dir}/deploy_key.pem`.
	///
	/// Reads the outputs every block emits:
	/// - `public_address`: the instance IP or hostname
	/// - `ssh_user`: the SSH username
	/// - `ssh_private_key`: the PEM-encoded private key
	pub async fn from_project(
		project: &terra::Project,
		port: u16,
	) -> Result<SshConnection> {
		let host = project.output("public_address").await?;
		let user = project.output("ssh_user").await?;
		let key_pem = project.output("ssh_private_key").await?;

		// the key file is what `ssh -i` reads, and ssh refuses a group- or
		// world-readable one outright
		let key_path = project.work_dir().join("deploy_key.pem");
		fs_ext::write_async(&key_path, key_pem.as_bytes()).await?;
		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			std::fs::set_permissions(
				key_path.as_path(),
				std::fs::Permissions::from_mode(0o600),
			)?;
		}

		SshConnection {
			host,
			user,
			port,
			key_path: AbsPathBuf::new(key_path)?,
		}
		.xok()
	}
}
