//! SSH utility functions for remote instance operations.
//!
//! These are general-purpose utilities for SSH and SCP operations,
//! primarily used for manual debugging and inspection of deployed instances.
//! The deploy pipeline itself does not use these directly; instances
//! self-retrieve their artifacts from S3.
use beet_core::prelude::*;

/// Standard SSH options for connecting to instances.
/// Disables host key checking and uses a 30-second connection timeout.
pub const SSH_OPTS: [&str; 6] = [
	"-o",
	"StrictHostKeyChecking=no",
	"-o",
	"UserKnownHostsFile=/dev/null",
	"-o",
	"ConnectTimeout=30",
];

/// SSH connection details for a remote instance.
#[derive(Debug, Clone)]
pub struct SshConnection {
	/// The public IP or hostname of the instance.
	pub host: String,
	/// The SSH user, ie `ec2-user` or `ubuntu`.
	pub user: String,
	/// Path to the private key file on disk.
	pub key_path: AbsPathBuf,
}

impl SshConnection {
	/// The `user@host` string for SSH commands.
	pub fn remote_user(&self) -> String {
		format!("{}@{}", self.user, self.host)
	}

	/// Build SSH arguments including key and standard options.
	fn ssh_args(&self) -> Vec<SmolStr> {
		let mut args: Vec<SmolStr> = SSH_OPTS.iter().map(|s| SmolStr::from(*s)).collect();
		args.push("-i".into());
		args.push(self.key_path.display().to_string().into());
		args
	}

	/// Run a command on the remote instance via SSH.
	pub async fn run_command(&self, command: &str) -> Result {
		let mut args = self.ssh_args();
		args.push(self.remote_user().into());
		args.push(command.into());
		ChildProcess::new("ssh")
			.with_args(args)
			.run_async()
			.await?;
		Ok(())
	}

	/// Copy a local file to the remote instance via SCP.
	pub async fn scp_to(
		&self,
		local_path: &std::path::Path,
		remote_path: &str,
	) -> Result {
		let mut args = self.ssh_args();
		args.push(local_path.display().to_string().into());
		args.push(format!("{}:{}", self.remote_user(), remote_path).into());
		ChildProcess::new("scp")
			.with_args(args)
			.run_async()
			.await?;
		Ok(())
	}

	/// Wait for SSH to become available, retrying up to `max_attempts` times.
	pub async fn wait_for_ready(&self, max_attempts: u32) -> Result {
		for attempt in 1..=max_attempts {
			info!("waiting for SSH (attempt {attempt}/{max_attempts})...");
			let result = self.run_command("echo ready").await;
			if result.is_ok() {
				return Ok(());
			}
			time_ext::sleep_millis(10_000).await;
		}
		bevybail!(
			"failed to connect to {} after {max_attempts} attempts",
			self.host
		)
	}
}

/// Read SSH connection details from terraform outputs in the given work directory.
/// Returns an [`SshConnection`] with a temporary key file written to `{work_dir}/deploy_key.pem`.
///
/// Reads the following terraform outputs:
/// - `public_address`: the instance IP or hostname
/// - `ssh_user`: the SSH username
/// - `ssh_private_key`: the PEM-encoded private key
pub async fn connection_from_tofu_outputs(
	work_dir: &AbsPathBuf,
) -> Result<SshConnection> {
	// read terraform outputs
	let host = ChildProcess::new("tofu")
		.with_args(["output", "-raw", "public_address"])
		.with_cwd(work_dir.clone())
		.run_async_stdout()
		.await
		.map_err(|err| bevyhow!("failed to read public_address: {err}"))?
		.trim()
		.to_string();

	let user = ChildProcess::new("tofu")
		.with_args(["output", "-raw", "ssh_user"])
		.with_cwd(work_dir.clone())
		.run_async_stdout()
		.await
		.map_err(|err| bevyhow!("failed to read ssh_user: {err}"))?
		.trim()
		.to_string();

	let key_pem = ChildProcess::new("tofu")
		.with_args(["output", "-raw", "ssh_private_key"])
		.with_cwd(work_dir.clone())
		.run_async_stdout()
		.await
		.map_err(|err| bevyhow!("failed to read ssh_private_key: {err}"))?;

	// write key to temp file with restricted permissions
	let key_path = work_dir.join("deploy_key.pem");
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
		key_path: AbsPathBuf::new(key_path)?,
	}
	.xok()
}
