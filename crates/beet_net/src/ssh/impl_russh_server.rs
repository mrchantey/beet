use crate::ssh::*;
use beet_core::prelude::*;
use bytes::Bytes;
use russh::Channel;
use russh::ChannelId;
use russh::keys::PrivateKey;
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
	/// Sending on this channel tells the forward task to close the SSH channel.
	pub close_tx: async_channel::Sender<()>,
	pub username: Option<String>,
}

/// Beet async fn: binds the SSH server and enters the accept loop.
#[allow(unused)]
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
	let close_tx = info.close_tx;
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
				.observe_any(move |_: On<SshDisconnect>| {
					close_tx.try_send(()).ok();
				})
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
	// In debug builds use a constant key so the fingerprint stays stable
	// between restarts. In release builds generate a fresh random key.
	#[cfg(debug_assertions)]
	let host_key = PrivateKey::from_bytes(DEBUG_HOST_KEY_BYTES)
		.expect("failed to load debug SSH host key");
	#[cfg(not(debug_assertions))]
	let host_key =
		PrivateKey::random(&mut rand::rng(), russh::keys::Algorithm::Ed25519)
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

		let (close_tx, close_rx) = async_channel::bounded::<()>(1);

		// Forward bevy → SSH client in a dedicated tokio task.
		// Also monitors close_rx for server-initiated disconnect.
		let handle = session.handle();
		let channel_id = channel.id();
		tokio::spawn(async move {
			let exit_code = loop {
				tokio::select! {
					result = to_client_rx.recv() => {
						match result {
							Ok(SshData::Bytes(bytes)) => {
								if handle.data(channel_id, bytes).await.is_err() {
									break 0u32;
								}
							}
							Ok(SshData::Exit(code)) => break code,
							Err(_) => break 0u32,
						}
					}
					_ = close_rx.recv() => break 0u32,
				}
			};
			// Send exit status before closing so the SSH client exits cleanly (code 0).
			handle.exit_status_request(channel_id, exit_code).await.ok();
			handle.close(channel_id).await.ok();
		});

		self.new_conn_tx
			.send(NewConnectionInfo {
				to_client: to_client_tx,
				from_client: from_client_rx,
				close_tx,
				username: self.authenticated_user.clone(),
			})
			.await
			.map_err(|_| russh::Error::Disconnect)?;

		Ok(true)
	}

	async fn auth_none(
		&mut self,
		user: &str,
	) -> std::result::Result<Auth, Self::Error> {
		// Accept anonymous login when no credentials are required.
		if self.credentials.is_none() {
			self.authenticated_user = Some(user.to_owned());
			Ok(Auth::Accept)
		} else {
			Ok(Auth::Reject {
				proceed_with_methods: None,
				partial_success: false,
			})
		}
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

	async fn channel_close(
		&mut self,
		channel: ChannelId,
		_session: &mut Session,
	) -> std::result::Result<(), Self::Error> {
		// Drop the sender so the beet recv loop gets the disconnect signal.
		if self.channel_id == Some(channel) {
			self.from_client_tx.take();
		}
		Ok(())
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

/// Constant Ed25519 host key used in debug builds so the fingerprint stays
/// stable between server restarts, avoiding spurious "host key changed" errors.
///
/// Generated once and baked in — never use this in production.
#[cfg(debug_assertions)]
const DEBUG_HOST_KEY_BYTES: &[u8] = &[
	111, 112, 101, 110, 115, 115, 104, 45, 107, 101, 121, 45, 118, 49, 0, 0, 0,
	0, 4, 110, 111, 110, 101, 0, 0, 0, 4, 110, 111, 110, 101, 0, 0, 0, 0, 0, 0,
	0, 1, 0, 0, 0, 51, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53,
	49, 57, 0, 0, 0, 32, 84, 27, 126, 103, 97, 239, 55, 231, 171, 215, 60, 204,
	55, 206, 162, 184, 249, 15, 71, 215, 245, 215, 225, 75, 130, 173, 187, 182,
	181, 10, 210, 100, 0, 0, 0, 136, 221, 168, 235, 133, 221, 168, 235, 133, 0,
	0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 32, 84,
	27, 126, 103, 97, 239, 55, 231, 171, 215, 60, 204, 55, 206, 162, 184, 249,
	15, 71, 215, 245, 215, 225, 75, 130, 173, 187, 182, 181, 10, 210, 100, 0,
	0, 0, 64, 210, 4, 195, 187, 64, 1, 160, 227, 81, 37, 130, 221, 200, 21, 20,
	6, 189, 46, 189, 110, 242, 46, 67, 183, 141, 49, 192, 198, 153, 195, 61,
	43, 84, 27, 126, 103, 97, 239, 55, 231, 171, 215, 60, 204, 55, 206, 162,
	184, 249, 15, 71, 215, 245, 215, 225, 75, 130, 173, 187, 182, 181, 10, 210,
	100, 0, 0, 0, 0, 1, 2, 3, 4, 5,
];
