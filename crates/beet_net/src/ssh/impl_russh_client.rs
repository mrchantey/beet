use crate::ssh::*;
use beet_core::prelude::*;
use std::sync::Arc;
use std::time::Duration;

/// Connects to an SSH server, sets up the entity data flow, and triggers
/// [`SshSessionReady`] when ready.
pub(crate) async fn connect_and_setup_entity(
	entity: AsyncEntity,
	addr: String,
	user: Option<String>,
	password: Option<String>,
) -> Result {
	let (to_server_tx, to_server_rx) = async_channel::unbounded::<SshData>();
	let (from_server_tx, from_server_rx) =
		async_channel::unbounded::<SshData>();

	// Phase 1: connect and authenticate on the shared tokio runtime.
	async_ext::on_tokio(async move {
		let (session, channel) =
			connect_inner(&addr, user.as_deref(), password.as_deref()).await?;
		// Phase 2: fire-and-forget the data loop on tokio so we can return.
		tokio::spawn(run_data_loop(
			session,
			channel,
			to_server_rx,
			from_server_tx,
		));
		Ok(())
	})
	.await?;

	// Phase 3: wire up the bevy entity with inline data flow.
	let to_server_obs = to_server_tx.clone();
	entity.with(move |mut entity| {
		entity
			.observe_any(
				move |ev: On<SshDataSend>,
				      mut commands: AsyncCommands|
				      -> Result {
					let to_server = to_server_obs.clone();
					let data = ev.event().clone();
					commands.run_local(async move |_| {
						to_server
							.send(data.take())
							.await
							.unwrap_or_else(|err| error!("{:?}", err));
					});
					Ok(())
				},
			)
			.run_async_local(async move |entity| {
				while let Ok(data) = from_server_rx.recv().await {
					entity.trigger_target_then(SshDataRecv(data)).await;
				}
				// channel closed — session ended
			})
			.trigger_target(SshSessionReady);
	});

	Ok(())
}

/// Low-level connect helper — just connects and authenticates, returns Ok/Err.
///
/// Used in tests to verify credential rejection without entity setup.
pub(crate) async fn test_connect(
	addr: impl AsRef<str>,
	user: Option<&str>,
	password: Option<&str>,
) -> Result<()> {
	let addr = addr.as_ref().to_owned();
	let user = user.map(str::to_owned);
	let password = password.map(str::to_owned);
	async_ext::on_tokio(async move {
		connect_inner(&addr, user.as_deref(), password.as_deref())
			.await
			.map(|_| ())
	})
	.await
}

/// Connects and authenticates, returning the session and an open channel.
async fn connect_inner(
	addr: &str,
	user: Option<&str>,
	password: Option<&str>,
) -> Result<(
	russh::client::Handle<BeetClientHandler>,
	russh::Channel<russh::client::Msg>,
)> {
	let config = Arc::new(russh::client::Config {
		inactivity_timeout: Some(Duration::from_secs(30)),
		..Default::default()
	});

	let mut session = russh::client::connect(config, addr, BeetClientHandler)
		.await
		.map_err(|e| bevyhow!("SSH connect failed: {}", e))?;

	// Attempt password auth if credentials supplied, otherwise try anonymous.
	if let (Some(u), Some(p)) = (user, password) {
		let result = session
			.authenticate_password(u, p)
			.await
			.map_err(|e| bevyhow!("SSH auth error: {}", e))?;
		if !result.success() {
			bevybail!("SSH authentication rejected for user '{}'", u);
		}
	} else {
		// Try anonymous / no-auth (server must accept with no credentials).
		let result = session
			.authenticate_none(user.unwrap_or(""))
			.await
			.map_err(|e| bevyhow!("SSH auth error: {}", e))?;
		if !result.success() {
			bevybail!("SSH anonymous authentication rejected");
		}
	}

	let channel = session
		.channel_open_session()
		.await
		.map_err(|e| bevyhow!("Failed to open SSH channel: {}", e))?;

	Ok((session, channel))
}

/// Bidirectional data loop — runs until disconnect.
async fn run_data_loop(
	session: russh::client::Handle<BeetClientHandler>,
	mut channel: russh::Channel<russh::client::Msg>,
	to_server_rx: async_channel::Receiver<SshData>,
	from_server_tx: async_channel::Sender<SshData>,
) {
	loop {
		tokio::select! {
			// bevy → SSH server
			result = to_server_rx.recv() => {
				match result {
					Ok(SshData::Bytes(bytes)) => {
						channel.data(bytes.as_ref()).await.ok();
					}
					Ok(SshData::Exit(_)) | Err(_) => break,
				}
			},
			// SSH server → bevy
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
					Some(ChannelMsg::Eof)
					| Some(ChannelMsg::Close)
					| None => break,
					_ => {}
				}
			},
		}
	}

	session
		.disconnect(russh::Disconnect::ByApplication, "", "English")
		.await
		.ok();
}

/// Minimal client handler — accepts all server keys (demo only).
struct BeetClientHandler;

impl russh::client::Handler for BeetClientHandler {
	type Error = russh::Error;

	async fn check_server_key(
		&mut self,
		_server_public_key: &russh::keys::ssh_key::PublicKey,
	) -> std::result::Result<bool, Self::Error> {
		Ok(true)
	}
}
