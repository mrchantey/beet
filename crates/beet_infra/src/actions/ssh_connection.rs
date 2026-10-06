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
	pub key_path: AbsPath,
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
		args.push(self.key_path.to_string().into());
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
		local_path: &AbsPath,
		remote_path: &str,
	) -> Result {
		let mut args = self.ssh_args();
		args.push("-P".into());
		args.push(self.port.to_string().into());
		args.push(local_path.to_string().into());
		args.push(format!("{}:{}", self.remote_user(), remote_path).into());
		ChildProcess::new("scp").with_args(args).run_async().await?;
		Ok(())
	}

	/// Forward `local_port` on this machine to `remote_port` on the box's
	/// loopback, for a service the security group deliberately does not admit
	/// from the internet (the mail box's management endpoint on 8080).
	///
	/// The returned handle kills the forward on drop, so a caller cannot leave
	/// a port open past the step that needed it. `-N` runs no remote command,
	/// and `ExitOnForwardFailure` turns a port already in use into an immediate
	/// error rather than a live ssh session forwarding nothing.
	pub async fn tunnel(
		&self,
		local_port: u16,
		remote_port: u16,
	) -> Result<ChildHandle> {
		let mut args = self.ssh_args();
		args.push("-o".into());
		args.push("ExitOnForwardFailure=yes".into());
		args.push("-N".into());
		args.push("-p".into());
		args.push(self.port.to_string().into());
		args.push("-L".into());
		args.push(
			format!("{local_port}:127.0.0.1:{remote_port}")
				.as_str()
				.into(),
		);
		args.push(self.remote_user().into());
		let handle = ChildProcess::new("ssh").with_args(args).spawn()?;
		info!(
			"tunnelling localhost:{local_port} to {}:{remote_port}",
			self.host
		);
		Ok(handle)
	}

	/// Wait for SSH to become available, retrying every `poll` until `timeout`
	/// elapses. At least one attempt is always made.
	///
	/// A missing identity file is refused before the first attempt, since no
	/// amount of waiting supplies one, and the final failure carries the last
	/// attempt's error rather than only a count.
	pub async fn wait_for_ready(
		&self,
		timeout: Duration,
		poll: Duration,
	) -> Result {
		if !fs_ext::exists(&self.key_path)? {
			bevybail!(
				"no private key at {}: the box admits only the public half \
				 its block declares, so put the matching private key there, \
				 or declare this device's public key and redeploy",
				self.key_path
			);
		}
		let max_attempts = (timeout.as_secs() / poll.as_secs().max(1)).max(1);
		let mut last_err = None;
		for attempt in 1..=max_attempts {
			info!(
				"waiting for ssh on {}:{} (attempt {attempt}/{max_attempts})...",
				self.host, self.port
			);
			match self.run_command("echo ready").await {
				Ok(_) => return Ok(()),
				Err(err) => last_err = Some(err),
			}
			time_ext::sleep(poll).await;
		}
		bevybail!(
			"failed to connect to {}:{} after {max_attempts} attempts, the \
			 last with: {}",
			self.host,
			self.port,
			last_err.map(|err| err.to_string()).unwrap_or_default()
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
		fs_ext::write_private(&key_path, key_pem.as_bytes())?;

		SshConnection {
			host,
			user,
			port,
			key_path,
		}
		.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A missing identity file is an error naming it, at once, rather than
	/// every attempt failing auth until the timeout.
	#[beet_core::test]
	async fn a_missing_key_is_refused_before_waiting() {
		let dir = TempDir::new().unwrap();
		let key_path = dir.path().join("absent_key");
		SshConnection {
			host: "192.0.2.1".into(),
			user: "ec2-user".into(),
			port: 22,
			key_path: key_path.clone(),
		}
		.wait_for_ready(Duration::from_secs(600), Duration::from_secs(5))
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains(&format!("no private key at {key_path}"));
	}
}
