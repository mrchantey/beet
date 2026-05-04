use crate::ssh::*;
use beet_core::prelude::*;
use std::sync::Arc;
use std::time::Duration;

/// Connects to an SSH server using password authentication.
///
/// Runs the russh client in a dedicated tokio thread, bridging
/// data flow via runtime-agnostic [`async_channel`] pairs.
pub(crate) async fn connect_russh(
	addr: impl AsRef<str>,
	user: &str,
	password: &str,
) -> Result<SshSession> {
	let addr = addr.as_ref().to_owned();
	let user = user.to_owned();
	let password = password.to_owned();

	let (to_server_tx, to_server_rx) = async_channel::unbounded::<SshData>();
	let (from_server_tx, from_server_rx) =
		async_channel::unbounded::<SshData>();

	// signal readiness: Ok(()) on connect, Err on failure
	let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();

	std::thread::spawn(move || {
		let rt = tokio::runtime::Runtime::new()
			.expect("failed to create tokio runtime for SSH client");
		rt.block_on(run_russh_client(
			addr,
			user,
			password,
			to_server_rx,
			from_server_tx,
			ready_tx,
		));
	});

	// wait for the background thread to confirm connection
	ready_rx
		.recv()
		.map_err(|_| bevyhow!("SSH client thread exited unexpectedly"))?
		.map_err(|e| bevyhow!("SSH connection failed: {}", e))?;

	Ok(SshSession {
		to_server: to_server_tx,
		from_server: from_server_rx,
	})
}

/// Runs the russh client connection and data loop inside a tokio runtime.
async fn run_russh_client(
	addr: String,
	user: String,
	password: String,
	to_server_rx: async_channel::Receiver<SshData>,
	from_server_tx: async_channel::Sender<SshData>,
	ready_tx: std::sync::mpsc::Sender<Result<(), String>>,
) {
	let config = Arc::new(russh::client::Config {
		inactivity_timeout: Some(Duration::from_secs(30)),
		..Default::default()
	});

	// connect and authenticate
	let mut session =
		match russh::client::connect(config, &addr, BeetClientHandler).await {
			Ok(s) => s,
			Err(err) => {
				ready_tx.send(Err(err.to_string())).ok();
				return;
			}
		};

	let auth = session.authenticate_password(user, password).await;
	match auth {
		Ok(result) if result.success() => {}
		Ok(_) => {
			ready_tx.send(Err("authentication rejected".into())).ok();
			return;
		}
		Err(err) => {
			ready_tx.send(Err(err.to_string())).ok();
			return;
		}
	}

	// open a session channel
	let mut channel = match session.channel_open_session().await {
		Ok(ch) => ch,
		Err(err) => {
			ready_tx.send(Err(err.to_string())).ok();
			return;
		}
	};

	// signal successful connection before entering the data loop
	ready_tx.send(Ok(())).ok();

	// bidirectional data loop
	loop {
		tokio::select! {
			// bevy → server: forward data through the SSH channel
			result = to_server_rx.recv() => {
				match result {
					Ok(SshData::Bytes(bytes)) => {
						channel.data(bytes.as_ref()).await.ok();
					}
					Ok(SshData::Exit(_)) | Err(_) => break,
				}
			},
			// server → bevy: receive channel messages
			msg = channel.wait() => {
				use russh::ChannelMsg;
				match msg {
					Some(ChannelMsg::Data { data }) => {
						from_server_tx
							.send(SshData::bytes(data.clone()))
							.await
							.ok();
					}
					Some(ChannelMsg::ExitStatus { exit_status }) => {
						from_server_tx
							.send(SshData::Exit(exit_status))
							.await
							.ok();
						break;
					}
					Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => {
						break;
					}
					_ => {}
				}
			},
		}
	}

	// cleanly disconnect
	session
		.disconnect(russh::Disconnect::ByApplication, "", "English")
		.await
		.ok();
}

/// Minimal russh client event handler (accepts all server keys for demo use).
struct BeetClientHandler;

impl russh::client::Handler for BeetClientHandler {
	type Error = russh::Error;

	async fn check_server_key(
		&mut self,
		_server_public_key: &russh::keys::ssh_key::PublicKey,
	) -> std::result::Result<bool, Self::Error> {
		// accept all server keys for demo purposes
		Ok(true)
	}
}
