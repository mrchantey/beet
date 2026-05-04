use crate::ssh::*;
use beet_core::prelude::*;
use bytes::Bytes;
use russh::Channel;
use russh::ChannelId;
use russh::server::Auth;
use russh::server::Msg;
use russh::server::Server as RusshServer;
use russh::server::Session;
use std::sync::Arc;
use std::time::Duration;

/// Info passed from the russh tokio task to the beet accept loop per connection.
pub(crate) struct NewConnectionInfo {
	pub to_client: async_channel::Sender<SshData>,
	pub from_client: async_channel::Receiver<SshData>,
	pub username: Option<String>,
}

/// Beet async fn: binds the SSH server and enters the accept loop.
pub(crate) async fn start_russh_server(entity: AsyncEntity) -> Result {
	let addr = entity.get::<SshServer, _>(|s| s.local_address()).await?;
	let listener = std::net::TcpListener::bind(&addr).map_err(|e| {
		bevyhow!("Failed to bind SSH server to {}: {}", addr, e)
	})?;
	start_russh_server_with_tcp(entity, listener).await
}

/// Like [`start_russh_server`] but accepts a pre-bound TCP listener.
pub(crate) async fn start_russh_server_with_tcp(
	entity: AsyncEntity,
	listener: std::net::TcpListener,
) -> Result {
	let addr = listener
		.local_addr()
		.map_err(|e| bevyhow!("Failed to get local address: {}", e))?;

	let credentials = entity
		.get::<SshServer, _>(|s| s.credentials.clone())
		.await?;

	let (new_conn_tx, new_conn_rx) =
		async_channel::unbounded::<NewConnectionInfo>();

	// Spawn the russh server on the shared tokio runtime (fire-and-forget).
	async_ext::tokio().spawn(run_russh_server_inner(
		listener,
		new_conn_tx,
		credentials,
	));

	info!("SSH server listening on {}", addr);

	// Accept loop: receive connections from the russh task and set up entities.
	loop {
		match new_conn_rx.recv().await {
			Ok(info) => {
				// Use fire-and-forget so the accept loop is not blocked.
				entity.with(move |mut server| {
					server.run_async_local(|server| {
						handle_connection(server, info)
					});
				});
			}
			Err(_) => break, // channel closed — server shutting down
		}
	}
	Ok(())
}

/// Spawns child entity with [`SshPeerInfo`] and wires up bidirectional data flow.
async fn handle_connection(
	server: AsyncEntity,
	info: NewConnectionInfo,
) -> Result {
	let server_id = server.id();
	let to_client = info.to_client;
	let from_client = info.from_client;
	let username = info.username;

	// Clone for the send observer (the recv loop takes the original).
	let to_client_obs = to_client.clone();

	// Spawn child entity and return its ID so we can trigger SshClientConnected on it.
	let child_id = server
		.world()
		.with_then(move |world: &mut World| -> Entity {
			let mut entity_mut =
				world.spawn((SshPeerInfo { username }, ChildOf(server_id)));
			let child_id = entity_mut.id();
			entity_mut
				.observe_any(
					move |ev: On<SshDataSend>,
					      mut commands: AsyncCommands|
					      -> Result {
						let to_client = to_client_obs.clone();
						let data = ev.event().clone();
						commands.run_local(async move |_| {
							to_client
								.send(data.take())
								.await
								.unwrap_or_else(|err| error!("{:?}", err));
						});
						Ok(())
					},
				)
				.run_async_local(async move |child_entity| {
					while let Ok(data) = from_client.recv().await {
						child_entity
							.trigger_target_then(SshDataRecv(data))
							.await;
					}
					child_entity
						.trigger_target_then(SshClientDisconnected)
						.await;
				});
			child_id
		})
		.await;

	// Trigger SshClientConnected on the child entity — auto_propagate carries it
	// up to the server entity, so server observers get original_target() == child.
	server
		.world()
		.entity(child_id)
		.trigger_target_then(SshClientConnected)
		.await;
	Ok(())
}

/// Runs the russh server loop inside the shared tokio runtime.
async fn run_russh_server_inner(
	listener: std::net::TcpListener,
	new_conn_tx: async_channel::Sender<NewConnectionInfo>,
	credentials: Option<SshCredentials>,
) {
	let host_key = russh::keys::PrivateKey::random(
		&mut rand::rng(),
		russh::keys::Algorithm::Ed25519,
	)
	.expect("failed to generate SSH host key");

	let config = Arc::new(russh::server::Config {
		inactivity_timeout: Some(Duration::from_secs(3600)),
		auth_rejection_time: Duration::from_secs(3),
		auth_rejection_time_initial: Some(Duration::from_secs(0)),
		keys: vec![host_key],
		..Default::default()
	});

	// tokio requires the socket to be in non-blocking mode
	listener
		.set_nonblocking(true)
		.expect("failed to set non-blocking mode");
	let tokio_listener = tokio::net::TcpListener::from_std(listener)
		.expect("failed to convert listener to tokio");

	let mut app = BeetSshApp {
		new_conn_tx,
		credentials,
	};
	if let Err(err) = app.run_on_socket(config, &tokio_listener).await {
		error!("SSH server error: {:?}", err);
	}
}

/// Per-app state shared across connection handlers.
#[derive(Clone)]
struct BeetSshApp {
	new_conn_tx: async_channel::Sender<NewConnectionInfo>,
	credentials: Option<SshCredentials>,
}

impl RusshServer for BeetSshApp {
	type Handler = BeetSshHandler;

	fn new_client(
		&mut self,
		_addr: Option<std::net::SocketAddr>,
	) -> BeetSshHandler {
		BeetSshHandler {
			new_conn_tx: self.new_conn_tx.clone(),
			credentials: self.credentials.clone(),
			authenticated_user: None,
			from_client_tx: None,
			channel_id: None,
		}
	}

	fn handle_session_error(
		&mut self,
		error: <Self::Handler as russh::server::Handler>::Error,
	) {
		// UnexpectedEof is normal when a client disconnects abruptly.
		match &error {
			russh::Error::IO(e)
				if e.kind() == std::io::ErrorKind::UnexpectedEof =>
			{
				debug!("SSH client disconnected (eof)");
			}
			_ => error!("SSH session error: {:?}", error),
		}
	}
}

/// Per-connection handler state.
struct BeetSshHandler {
	new_conn_tx: async_channel::Sender<NewConnectionInfo>,
	credentials: Option<SshCredentials>,
	authenticated_user: Option<String>,
	from_client_tx: Option<async_channel::Sender<SshData>>,
	channel_id: Option<ChannelId>,
}

impl russh::server::Handler for BeetSshHandler {
	type Error = russh::Error;

	async fn channel_open_session(
		&mut self,
		channel: Channel<Msg>,
		session: &mut Session,
	) -> std::result::Result<bool, Self::Error> {
		let (to_client_tx, to_client_rx) =
			async_channel::unbounded::<SshData>();
		let (from_client_tx, from_client_rx) =
			async_channel::unbounded::<SshData>();

		self.from_client_tx = Some(from_client_tx);
		self.channel_id = Some(channel.id());

		// Forward bevy → SSH client in a dedicated tokio task.
		let handle = session.handle();
		let channel_id = channel.id();
		tokio::spawn(async move {
			while let Ok(data) = to_client_rx.recv().await {
				if let SshData::Bytes(bytes) = data {
					handle.data(channel_id, bytes).await.ok();
				}
			}
		});

		self.new_conn_tx
			.send(NewConnectionInfo {
				to_client: to_client_tx,
				from_client: from_client_rx,
				username: self.authenticated_user.clone(),
			})
			.await
			.map_err(|_| russh::Error::Disconnect)?;

		Ok(true)
	}

	async fn auth_publickey(
		&mut self,
		_user: &str,
		_key: &russh::keys::ssh_key::PublicKey,
	) -> std::result::Result<Auth, Self::Error> {
		// Accept public key auth only when no password credentials are required.
		if self.credentials.is_none() {
			Ok(Auth::Accept)
		} else {
			Ok(Auth::Reject {
				proceed_with_methods: None,
				partial_success: false,
			})
		}
	}

	async fn auth_password(
		&mut self,
		user: &str,
		password: &str,
	) -> std::result::Result<Auth, Self::Error> {
		let accepted = match &self.credentials {
			None => true,
			Some(creds) => creds.username == user && creds.password == password,
		};
		if accepted {
			self.authenticated_user = Some(user.to_owned());
			Ok(Auth::Accept)
		} else {
			Ok(Auth::Reject {
				proceed_with_methods: None,
				partial_success: false,
			})
		}
	}

	async fn data(
		&mut self,
		_channel: ChannelId,
		data: &[u8],
		_session: &mut Session,
	) -> std::result::Result<(), Self::Error> {
		if let Some(tx) = &self.from_client_tx {
			tx.send(SshData::bytes(Bytes::copy_from_slice(data)))
				.await
				.ok();
		}
		Ok(())
	}
}
