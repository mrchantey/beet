//! Deploy step for Lightsail instances via SCP and SSH.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Deploys the built binary to a Lightsail instance.
/// Reads the static IP and SSH private key from terraform outputs,
/// SCPs the binary, and restarts the systemd service.
#[action]
#[derive(Default, Component)]
pub async fn DeployLightsailAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	// build the terraform project to access outputs
	let (project, app_name) = cx
		.caller
		.with_state::<StackQuery, _>(|entity, query| -> Result<_> {
			let project = query.build_project(entity)?;
			let app_name = project.app_name().to_string();
			(project, app_name).xok()
		})
		.await?;

	// ensure init has been run so we can read outputs
	project.apply().await?;

	let dir = project.work_directory().into_abs();

	// read the static IP from terraform output
	let ip = ChildProcess::new("tofu")
		.with_args(&["output", "-raw", "static_ip_address"])
		.with_cwd(&dir)
		.run_async_stdout()
		.await
		.map_err(|err| bevyhow!("failed to read static_ip_address: {err}"))?;
	let ip = ip.trim().to_string();

	// read the SSH private key from terraform output
	let key_pem = ChildProcess::new("tofu")
		.with_args(&["output", "-raw", "ssh_private_key"])
		.with_cwd(&dir)
		.run_async_stdout()
		.await
		.map_err(|err| bevyhow!("failed to read ssh_private_key: {err}"))?;

	// save key to temp file
	let key_path = dir.join("deploy_key.pem");
	fs_ext::write_async(&key_path, key_pem.as_bytes()).await?;
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		std::fs::set_permissions(
			key_path.as_path(),
			std::fs::Permissions::from_mode(0o600),
		)?;
	}
	let key_str = key_path.display().to_string();

	// find the built binary
	let exe_path = cx
		.caller
		.with_state::<AncestorQuery<&CargoBuildCmd>, _>(|entity, query| {
			query.get(entity).map(|cmd| cmd.exe_path(None))
		})
		.await?;
	let exe_str = exe_path.display().to_string();

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

	// wait for instance SSH to be ready (retry a few times)
	let mut connected = false;
	for attempt in 1..=10 {
		info!("waiting for SSH (attempt {attempt}/10)...");
		let result = ChildProcess::new("ssh")
			.with_args(
				&[&ssh_opts[..], &[&format!("ec2-user@{ip}"), "echo", "ready"]]
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

	// SCP the binary to /opt/{app_name}/app
	let remote_path = format!("ec2-user@{ip}:/tmp/app_binary");
	ChildProcess::new("scp")
		.with_args(&[&ssh_opts[..], &[&exe_str, &remote_path]].concat())
		.run_async()
		.await
		.map_err(|err| bevyhow!("SCP failed: {err}"))?;

	// move binary into place and restart service
	let install_cmd = format!(
		"sudo mv /tmp/app_binary /opt/{app_name}/app && \
		 sudo chmod +x /opt/{app_name}/app && \
		 sudo systemctl restart {app_name}.service"
	);
	ChildProcess::new("ssh")
		.with_args(
			&[&ssh_opts[..], &[&format!("ec2-user@{ip}"), &install_cmd]]
				.concat(),
		)
		.run_async()
		.await
		.map_err(|err| bevyhow!("SSH restart failed: {err}"))?;

	// clean up the key file
	fs_ext::remove_async(&key_path).await?;

	info!("deployed to {ip} successfully");
	Pass(cx.input).xok()
}
