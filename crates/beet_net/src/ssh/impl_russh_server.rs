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

/// Message sent from the russh tokio thread to the beet accept loop.
pub(crate) struct NewConnectionInfo {
	/// Send data to this client from bevy.
	pub to_client: async_channel::Sender<SshData>,
	/// Receive data from this client in bevy.
	pub from_client: async_channel::Receiver<SshData>,
}

/// Beet async fn: starts the SSH server and accepts connections.
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

	// channel for russh thread to notify beet of new connections
	let (new_conn_tx, new_conn_rx) =
		async_channel::unbounded::<NewConnectionInfo>();

	// run russh in a dedicated tokio thread
	std::thread::spawn(move || {
		let rt = tokio::runtime::Runtime::new()
			.expect("failed to create tokio runtime for SSH server");
		rt.block_on(run_russh_server_with_tcp_inner(listener, new_conn_tx));
	});

	info!("SSH server listening on {}", addr);

	// beet accept loop: receive connections from the russh thread
	loop {
		match new_conn_rx.recv().await {
			Ok(info) => {
				// spawn a child entity per connection
				entity
					.spawn_child(SshConnection {
						to_client: info.to_client,
						from_client: info.from_client,
					})
					.await;
				entity.trigger_target_then(SshClientConnected).await;
			}
			Err(_) => {
				// channel closed, server is shutting down
				break;
			}
		}
	}
	Ok(())
}

/// Runs the russh server inside a tokio runtime (blocks until shutdown).
async fn run_russh_server_with_tcp_inner(
	listener: std::net::TcpListener,
	new_conn_tx: async_channel::Sender<NewConnectionInfo>,
) {
	// generate ephemeral host key
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

	let tokio_listener = {
		// tokio requires the socket to be in non-blocking mode
		listener
			.set_nonblocking(true)
			.expect("failed to set non-blocking mode");
		tokio::net::TcpListener::from_std(listener)
			.expect("failed to convert listener to tokio")
	};

	let mut app = BeetSshApp { new_conn_tx, id: 0 };
	if let Err(err) = app.run_on_socket(config, &tokio_listener).await {
		error!("SSH server error: {:?}", err);
	}
}

/// Per-app state shared across connection handlers.
#[derive(Clone)]
struct BeetSshApp {
	new_conn_tx: async_channel::Sender<NewConnectionInfo>,
	id: usize,
}

impl RusshServer for BeetSshApp {
	type Handler = BeetSshHandler;

	fn new_client(
		&mut self,
		_addr: Option<std::net::SocketAddr>,
	) -> BeetSshHandler {
		let handler = BeetSshHandler {
			new_conn_tx: self.new_conn_tx.clone(),
			from_client_tx: None,
			channel_id: None,
		};
		self.id += 1;
		handler
	}

	fn handle_session_error(
		&mut self,
		error: <Self::Handler as russh::server::Handler>::Error,
	) {
		error!("SSH session error: {:?}", error);
	}
}

/// Per-connection handler state.
struct BeetSshHandler {
	new_conn_tx: async_channel::Sender<NewConnectionInfo>,
	/// Sends client data to the bevy world.
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

		// tokio task: forward bevy → russh client
		let handle = session.handle();
		let channel_id = channel.id();
		tokio::spawn(async move {
			while let Ok(data) = to_client_rx.recv().await {
				if let SshData::Bytes(bytes) = data {
					// ignore send errors (client disconnected)
					handle.data(channel_id, bytes).await.ok();
				}
			}
		});

		// notify beet of this new connection
		self.new_conn_tx
			.send(NewConnectionInfo {
				to_client: to_client_tx,
				from_client: from_client_rx,
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
		Ok(Auth::Accept)
	}

	async fn auth_password(
		&mut self,
		_user: &str,
		_password: &str,
	) -> std::result::Result<Auth, Self::Error> {
		Ok(Auth::Accept)
	}

	async fn data(
		&mut self,
		_channel: ChannelId,
		data: &[u8],
		_session: &mut Session,
	) -> std::result::Result<(), Self::Error> {
		if let Some(tx) = &self.from_client_tx {
			// forward client data to the bevy world
			tx.send(SshData::bytes(Bytes::copy_from_slice(data)))
				.await
				.ok();
		}
		Ok(())
	}
}
