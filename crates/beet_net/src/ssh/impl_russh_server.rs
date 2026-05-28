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
	pub to_client: async_channel::Sender<SshEvent>,
	pub from_client: async_channel::Receiver<SshEvent>,
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
				// Spawn handling so the accept loop is not blocked; awaiting
				// `with` only runs the spawn, not the connection handler.
				entity
					.with(move |mut server| {
						server.run_async_local(|server| {
							handle_connection(server, info)
						});
					})
					.await;
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
	let username = info.username.map(|u| u.into());

	// Clone for the send observer (the recv loop takes the original).
	let to_client_obs = to_client.clone();

	// Spawn child entity and return its ID so we can trigger SshRecv(Connect) on it.
	let child_id = server
		.world()
		.with_then(move |world: &mut World| -> Entity {
			let mut entity_mut =
				world.spawn((SshPeerInfo { username }, ChildOf(server_id)));
			let child_id = entity_mut.id();
			entity_mut
				.observe_any(
					move |ev: On<SshSend>,
					      commands: AsyncCommands|
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
					while let Ok(event) = from_client.recv().await {
						child_entity
							.trigger_target_then(SshRecv(event))
							.await
							.ok();
					}
					// Channel closed — fire a Close event so observers can clean up.
					child_entity
						.trigger_target_then(SshRecv(SshEvent::Close(None)))
						.await
						.ok();
				});
			child_id
		})
		.await;

	// Trigger SshRecv(Connect) on the child entity — auto_propagate carries it
	// up to the server entity, so server observers get original_target() == child.
	server
		.world()
		.entity(child_id)
		.trigger_target_then(SshRecv(SshEvent::Connect))
		.await?;
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
	from_client_tx: Option<async_channel::Sender<SshEvent>>,
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
			async_channel::unbounded::<SshEvent>();
		let (from_client_tx, from_client_rx) =
			async_channel::unbounded::<SshEvent>();

		self.from_client_tx = Some(from_client_tx);
		self.channel_id = Some(channel.id());

		// Forward bevy → SSH client in a dedicated tokio task.
		// SshEvent::Close signals a server-initiated disconnect.
		let handle = session.handle();
		let channel_id = channel.id();
		tokio::spawn(async move {
			let exit_code = loop {
				match to_client_rx.recv().await {
					Ok(SshEvent::Data(bytes)) => {
						if handle.data(channel_id, bytes).await.is_err() {
							break 0u32;
						}
					}
					Ok(SshEvent::Close(frame)) => {
						break frame.map(|f| f.code).unwrap_or(0);
					}
					Err(_) => break 0u32,
					// Ignore other event types on the send path
					_ => {}
				}
			};
			// Send exit status before closing so the SSH client exits cleanly.
			handle.exit_status_request(channel_id, exit_code).await.ok();
			handle.close(channel_id).await.ok();
		});

		self.new_conn_tx
			.send(NewConnectionInfo {
				to_client: to_client_tx,
				from_client: from_client_rx,
				username: self.authenticated_user.clone(),
			})
			.await
			.map_err(|err| {
				error!(
					"Failed to send new connection info to accept loop: {:?}",
					err
				);
				russh::Error::Disconnect
			})?;

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

	async fn pty_request(
		&mut self,
		_channel: ChannelId,
		term: &str,
		col_width: u32,
		row_height: u32,
		pix_width: u32,
		pix_height: u32,
		_modes: &[(russh::Pty, u32)],
		_session: &mut Session,
	) -> std::result::Result<(), Self::Error> {
		if let Some(tx) = &self.from_client_tx {
			tx.send(SshEvent::RequestPty(RequestPty {
				terminal: term.into(),
				window: SshWindowSize {
					cells: UVec2::new(col_width, row_height),
					pixels: UVec2::new(pix_width, pix_height),
				},
				terminal_modes: Vec::new(),
			}))
			.await
			.ok();
		}
		Ok(())
	}

	async fn window_change_request(
		&mut self,
		_channel: ChannelId,
		col_width: u32,
		row_height: u32,
		pix_width: u32,
		pix_height: u32,
		_session: &mut Session,
	) -> std::result::Result<(), Self::Error> {
		if let Some(tx) = &self.from_client_tx {
			tx.send(SshEvent::Resize(SshWindowSize {
				cells: UVec2::new(col_width, row_height),
				pixels: UVec2::new(pix_width, pix_height),
			}))
			.await
			.ok();
		}
		Ok(())
	}

	async fn shell_request(
		&mut self,
		_channel: ChannelId,
		_session: &mut Session,
	) -> std::result::Result<(), Self::Error> {
		if let Some(tx) = &self.from_client_tx {
			tx.send(SshEvent::RequestShell).await.ok();
		}
		Ok(())
	}

	async fn x11_request(
		&mut self,
		_channel: ChannelId,
		_single_connection: bool,
		x11_auth_protocol: &str,
		x11_auth_cookie: &str,
		x11_screen_number: u32,
		_session: &mut Session,
	) -> std::result::Result<(), Self::Error> {
		if let Some(tx) = &self.from_client_tx {
			tx.send(SshEvent::RequestX11(RequestX11 {
				auth_protocol: x11_auth_protocol.into(),
				auth_cookie: x11_auth_cookie.into(),
				screen: x11_screen_number,
			}))
			.await
			.ok();
		}
		Ok(())
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
			tx.send(SshEvent::Data(Bytes::copy_from_slice(data)))
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
