//! Deploy step for Lightsail instances via SCP and SSH.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Local directory to sync to the Lightsail instance during deployment.
/// Placed on an entity in the deploy sequence alongside [`LightsailBlock`].
#[derive(Debug, Clone, Component)]
pub struct LightsailAssets(WsPathBuf);

impl LightsailAssets {
	pub fn new(path: impl Into<WsPathBuf>) -> Self { Self(path.into()) }
	pub fn path(&self) -> &WsPathBuf { &self.0 }
}

/// Deploy a binary to a remote host via SCP and restart systemd service.
/// This is the core deploy logic used by both initial deployments
/// and rollback/rollforward re-deployments.
pub async fn deploy_binary_via_ssh(
	work_dir: &AbsPathBuf,
	app_name: &str,
	artifact_path: &std::path::Path,
	assets_dir: Option<&std::path::Path>,
) -> Result {
	// read public address from terraform output
	let ip = ChildProcess::new("tofu")
		.with_args(["output", "-raw", "public_address"])
		.with_cwd(work_dir.clone())
		.run_async_stdout()
		.await
		.map_err(|err| bevyhow!("failed to read public_address: {err}"))?;
	let ip = ip.trim().to_string();

	// read SSH user from terraform output
	let ssh_user = ChildProcess::new("tofu")
		.with_args(["output", "-raw", "ssh_user"])
		.with_cwd(work_dir.clone())
		.run_async_stdout()
		.await
		.map_err(|err| bevyhow!("failed to read ssh_user: {err}"))?;
	let ssh_user = ssh_user.trim().to_string();
	let remote_user = format!("{ssh_user}@{ip}");

	// read SSH private key from terraform output
	let key_pem = ChildProcess::new("tofu")
		.with_args(["output", "-raw", "ssh_private_key"])
		.with_cwd(work_dir.clone())
		.run_async_stdout()
		.await
		.map_err(|err| bevyhow!("failed to read ssh_private_key: {err}"))?;

	// save key to temp file with restricted permissions
	let key_path = work_dir.join("deploy_key.pem");
	fs_ext::write_async(&key_path, key_pem.as_bytes()).await?;
	// SSH requires private key files to have restricted permissions (owner read/write only).
	// Without this, ssh/scp will refuse to use the key with "Permissions too open" error.
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		std::fs::set_permissions(
			key_path.as_path(),
			std::fs::Permissions::from_mode(0o600),
		)?;
	}
	let key_str = key_path.display().to_string();
	let exe_str = artifact_path.display().to_string();

	info!("deploying to {ip} via SCP");

	let ssh_opts = [
		"-o",
		"StrictHostKeyChecking=no",
		"-o",
		"UserKnownHostsFile=/dev/null",
		"-o",
		"ConnectTimeout=30",
		"-i",
		&key_str,
	];

	// wait for instance SSH to be ready
	let mut connected = false;
	for attempt in 1..=10 {
		info!("waiting for SSH (attempt {attempt}/10)...");
		let result = ChildProcess::new("ssh")
			.with_args(
				[&ssh_opts[..], &[remote_user.as_str(), "echo", "ready"]]
					.concat(),
			)
			.run_async()
			.await;
		if result.is_ok() {
			connected = true;
			break;
		}
		time_ext::sleep_millis(10_000).await;
	}
	if !connected {
		bevybail!("failed to connect to instance at {ip} after 10 attempts");
	}

	// SCP the binary to remote host
	let remote_path = format!("{remote_user}:/tmp/app_binary");
	ChildProcess::new("scp")
		.with_args(
			[&ssh_opts[..], &[exe_str.as_str(), remote_path.as_str()]].concat(),
		)
		.run_async()
		.await
		.map_err(|err| bevyhow!("SCP failed: {err}"))?;

	// move binary into place
	let move_cmd = format!(
		"sudo mv /tmp/app_binary /opt/{app_name}/app && \\
		 sudo chmod +x /opt/{app_name}/app"
	);
	ChildProcess::new("ssh")
		.with_args(
			[&ssh_opts[..], &[remote_user.as_str(), move_cmd.as_str()]]
				.concat(),
		)
		.run_async()
		.await
		.map_err(|err| bevyhow!("SSH move binary failed: {err}"))?;

	// sync assets directory if provided
	if let Some(assets) = assets_dir {
		let assets_str = assets.display().to_string();
		// ensure trailing slash for rsync to copy contents into the directory
		let assets_src = if assets_str.ends_with('/') {
			assets_str
		} else {
			format!("{assets_str}/")
		};
		let remote_assets = format!("{remote_user}:/opt/{app_name}/assets/");
		// create remote directory
		let mkdir_cmd = format!(
			"sudo mkdir -p /opt/{app_name}/assets && sudo chown {ssh_user} /opt/{app_name}/assets"
		);
		ChildProcess::new("ssh")
			.with_args(
				[&ssh_opts[..], &[remote_user.as_str(), mkdir_cmd.as_str()]]
					.concat(),
			)
			.run_async()
			.await
			.map_err(|err| bevyhow!("SSH mkdir assets failed: {err}"))?;

		info!("syncing assets from {assets_src} to {remote_assets}");
		let rsync_ssh = format!(
			"ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=30 -i {}",
			key_str
		);
		ChildProcess::new("rsync")
			.with_args([
				"-az".into(),
				"-e".into(),
				SmolStr::from(&rsync_ssh),
				SmolStr::from(&assets_src),
				SmolStr::from(&remote_assets),
			])
			.run_async()
			.await
			.map_err(|err| bevyhow!("rsync assets failed: {err}"))?;
		info!("assets synced successfully");
	}

	// restart the service
	let restart_cmd = format!("sudo systemctl restart {app_name}.service");
	ChildProcess::new("ssh")
		.with_args(
			[&ssh_opts[..], &[remote_user.as_str(), restart_cmd.as_str()]]
				.concat(),
		)
		.run_async()
		.await
		.map_err(|err| bevyhow!("SSH restart failed: {err}"))?;

	// clean up the key file
	fs_ext::remove_async(&key_path).await?;

	info!("deployed to {ip} successfully");
	Ok(())
}

/// Deploys the built binary to a Lightsail instance.
/// Reads the public address, SSH user, and private key from terraform outputs,
/// SCPs the binary, and restarts the systemd service.
#[action]
#[derive(Default, Component)]
pub async fn DeployLightsailAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	// build the terraform project to access outputs
	let (dir, app_name) = cx
		.caller
		.with_state::<StackQuery, _>(|entity, query| -> Result<_> {
			let project = query.build_project(entity)?;
			let dir = project.work_directory().into_abs();
			let app_name = project.app_name().to_string();
			(dir, app_name).xok()
		})
		.await?;

	// find the built binary from stack descendants
	let exe_path = cx
		.caller
		.with_state::<StackQuery, _>(|entity, query| -> Result<_> {
			let artifacts = query.collect_artifacts(entity)?;
			artifacts
				.first()
				.map(|(build, _)| build.artifact_path().to_path_buf())
				.ok_or_else(|| {
					bevyhow!("no build artifact found in stack descendants")
				})
		})
		.await?;

	// find optional assets directory
	let assets_abs: Option<std::path::PathBuf> = cx
		.caller
		.with_state::<Query<&LightsailAssets>, _>(|_entity, query| {
			query
				.iter()
				.next()
				.map(|assets| assets.path().into_abs().into())
		})
		.await;

	deploy_binary_via_ssh(&dir, &app_name, &exe_path, assets_abs.as_deref())
		.await?;
	Pass(cx.input).xok()
}
