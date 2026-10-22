use crate::ssh::*;
use base64::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use russh::Channel;
use russh::ChannelId;
use russh::keys::PrivateKey;
use russh::server::Auth;
use russh::server::ChannelOpenHandle;
use russh::server::Msg;
use russh::server::Server as RusshServer;
use russh::server::Session;
use std::sync::Arc;
use std::time::Duration;

/// How long a server-closed session waits for its client to answer the close
/// before disconnecting it.
const CLOSE_GRACE: Duration = Duration::from_secs(5);

/// Info passed from the russh tokio task to the beet accept loop per connection.
pub(crate) struct NewConnectionInfo {
	pub to_client: async_channel::Sender<SshEvent>,
	pub from_client: async_channel::Receiver<SshEvent>,
	pub username: Option<String>,
	pub peer_addr: Option<std::net::SocketAddr>,
}

/// Beet async fn: binds the SSH server and enters the accept loop.
#[allow(unused)]
pub(crate) async fn start_russh_server(entity: AsyncEntity) -> Result {
	let addr = entity.get::<SshServer, _>(|s| s.bind_address()).await?;
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

	let (credentials, idle_timeout, pty_timeout, declared) = entity
		.get::<SshServer, _>(|server| {
			(
				server.credentials.clone(),
				server.idle_timeout,
				server.pty_timeout,
				server.host_key.clone(),
			)
		})
		.await?;
	// resolved before serving, so a declared key that cannot be read or parsed
	// fails the server rather than serving under a stranger's fingerprint
	let declared = match declared {
		Some(key) => Some(key),
		None => SshHostKey::resolve(&entity).await?,
	};
	let host_key = host_key(declared.as_ref())?;

	let (new_conn_tx, new_conn_rx) =
		async_channel::unbounded::<NewConnectionInfo>();

	// Spawn the russh server on the shared tokio runtime (fire-and-forget).
	async_ext::tokio().spawn(run_russh_server_inner(
		listener,
		new_conn_tx,
		credentials,
		idle_timeout,
		host_key,
	));

	info!("SSH server listening on {}", addr);

	// Accept loop: receive connections from the russh task and set up entities.
	loop {
		match new_conn_rx.recv().await {
			Ok(info) => {
				entity
					.run_async_local(move |server| {
						handle_connection(server, info, pty_timeout)
					})
					.await
					.ok();
			}
			Err(_) => break, // channel closed — server shutting down
		}
	}
	Ok(())
}

/// Spawns the connection entity with [`SshPeerInfo`] and wires up bidirectional
/// data flow, for the whole life of the connection.
async fn handle_connection(
	server: AsyncEntity,
	info: NewConnectionInfo,
	pty_timeout: Option<Duration>,
) {
	let server_id = server.id();
	let to_client = info.to_client;
	let from_client = info.from_client;
	let username = info.username.map(|u| u.into());
	let peer_addr = info.peer_addr;

	// Clone for the send observer (the recv loop takes the original).
	let to_client_obs = to_client.clone();

	// Spawn the connection entity, announce it, and start its recv loop, all in
	// one world scope: the loop cannot deliver a close (or despawn the entity)
	// before the connect it holds the world against, so a client that vanishes
	// immediately is still seen to connect before it is seen to go.
	server
		.world()
		.with(move |world: &mut World| {
			world
				.spawn((
					SshPeerInfo {
						username,
						peer_addr,
					},
					ChildOf(server_id),
					// the pty requirement, if this server sets one, starts ticking
					// from the moment the channel opens.
					OnSpawn::new(move |entity: &mut EntityWorldMut| {
						if let Some(timeout) = pty_timeout {
							entity.insert(RequirePty::new(timeout));
						}
					}),
				))
				.observe_any(
					move |ev: On<SshSend>, commands: AsyncCommands| -> Result {
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
				// auto_propagate carries this up to the server entity, so server
				// observers get original_target() == the connection.
				.trigger_target(SshRecv(SshEvent::Connect))
				.run_async_local(async move |child_entity| {
					while let Ok(event) = from_client.recv().await {
						child_entity.trigger_target(SshRecv(event)).await.ok();
					}
					// Channel closed — fire a Close event so observers can clean up,
					// then reclaim the connection: the accept loop spawned this
					// entity, so it owns the whole lifecycle. Despawning it drops the
					// send observer with it, ending the client's forwarding task, so a
					// disconnect leaves nothing behind for a long-running server.
					child_entity
						.trigger_target(SshRecv(SshEvent::Close(None)))
						.await
						.ok();
					child_entity.despawn().await.ok();
				});
		})
		.await;
}

/// Runs the russh server loop inside the shared tokio runtime.
async fn run_russh_server_inner(
	listener: std::net::TcpListener,
	new_conn_tx: async_channel::Sender<NewConnectionInfo>,
	credentials: Option<SshCredentials>,
	idle_timeout: Duration,
	host_key: PrivateKey,
) {
	let config = Arc::new(russh::server::Config {
		inactivity_timeout: Some(idle_timeout),
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

/// Selects the Ed25519 host key, in precedence order:
/// 1. the `declared` key ([`SshServer::host_key`], else its entity's
///    [`SshHostKeyRef`]), how a deployed box presents its stack's key.
/// 2. `BEET_SSH_HOST_KEY`, letting every local process present one
///    fingerprint, in any form [`parse_host_key`] reads.
/// 3. debug builds: the stable [`SshHostKey::DEBUG`].
/// 4. release builds: a fresh random key.
///
/// A key given at 1 or 2 that does not parse is an error: falling through
/// would serve a new fingerprint every start.
fn host_key(declared: Option<&SshHostKey>) -> Result<PrivateKey> {
	if let Some(key) = declared {
		return parse_host_key(key.encoded())
			.map_err(|err| bevyhow!("malformed declared ssh host key: {err}"));
	}
	if let Ok(value) = env_ext::var("BEET_SSH_HOST_KEY") {
		return parse_host_key(&value)
			.map_err(|err| bevyhow!("malformed BEET_SSH_HOST_KEY: {err}"));
	}
	#[cfg(debug_assertions)]
	return parse_host_key(SshHostKey::DEBUG);
	#[cfg(not(debug_assertions))]
	return PrivateKey::random(
		&mut rand::rng(),
		russh::keys::Algorithm::Ed25519,
	)?
	.xok();
}

/// Parses an OpenSSH private key from the file `ssh-keygen` writes (its PEM
/// armor), or from base64 of either that file or the binary encoding inside
/// it. Whitespace in base64 is ignored, so wrapped `base64` output reads too.
pub(crate) fn parse_host_key(value: &str) -> Result<PrivateKey> {
	const PEM_PREFIX: &str = "-----BEGIN";
	let value = value.trim();
	let bytes = if value.starts_with(PEM_PREFIX) {
		value.as_bytes().to_vec()
	} else {
		// base64's no_std DecodeError lacks a std::error::Error impl, so render
		// it into a message rather than relying on the `?`/`From<Error>` path.
		BASE64_STANDARD
			.decode(value.split_whitespace().collect::<String>())
			.map_err(|err| bevyhow!("{err}"))?
	};
	if bytes.starts_with(PEM_PREFIX.as_bytes()) {
		PrivateKey::from_openssh(&bytes)
	} else {
		PrivateKey::from_bytes(&bytes)
	}?
	.xok()
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
		addr: Option<std::net::SocketAddr>,
	) -> BeetSshHandler {
		BeetSshHandler {
			new_conn_tx: self.new_conn_tx.clone(),
			credentials: self.credentials.clone(),
			authenticated_user: None,
			from_client_tx: None,
			client_closed_tx: None,
			channel_id: None,
			peer_addr: addr,
		}
	}

	fn handle_session_error(
		&mut self,
		error: <Self::Handler as russh::server::Handler>::Error,
	) {
		// a client going away is normal, whether it says goodbye (`Disconnect`)
		// or just drops the socket (`UnexpectedEof`); neither is an error, and
		// logging them as one buries real faults in a deployed service's logs.
		match &error {
			russh::Error::Disconnect => debug!("SSH client disconnected"),
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
	/// Dropped when the client answers a close with its own (or the session
	/// ends), how the forwarding task learns the client hung up by itself.
	client_closed_tx: Option<async_channel::Sender<()>>,
	channel_id: Option<ChannelId>,
	/// The client's network address, forwarded by russh's `new_client`.
	peer_addr: Option<std::net::SocketAddr>,
}

impl russh::server::Handler for BeetSshHandler {
	type Error = russh::Error;

	async fn channel_open_session(
		&mut self,
		channel: Channel<Msg>,
		reply: ChannelOpenHandle,
		session: &mut Session,
	) -> std::result::Result<(), Self::Error> {
		let (to_client_tx, to_client_rx) =
			async_channel::unbounded::<SshEvent>();
		let (from_client_tx, from_client_rx) =
			async_channel::unbounded::<SshEvent>();

		let (client_closed_tx, client_closed_rx) =
			async_channel::bounded::<()>(1);
		self.from_client_tx = Some(from_client_tx);
		self.client_closed_tx = Some(client_closed_tx);
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
			handle.eof(channel_id).await.ok();
			handle.close(channel_id).await.ok();
			// One channel per connection, so the session ends with it. A client
			// answers the close with its own and hangs up, keeping its exit status
			// and every byte before it: a disconnect is fatal to OpenSSH, which
			// exits 255 and drops output it has not yet written. Only a peer that
			// ignores the close is disconnected, after a grace, which keeps a reap
			// decisive rather than waiting out the inactivity timeout.
			let answered =
				tokio::time::timeout(CLOSE_GRACE, client_closed_rx.recv())
					.await
					.is_ok();
			if !answered {
				handle
					.disconnect(
						russh::Disconnect::ByApplication,
						String::new(),
						String::new(),
					)
					.await
					.ok();
			}
		});

		self.new_conn_tx
			.send(NewConnectionInfo {
				to_client: to_client_tx,
				from_client: from_client_rx,
				username: self.authenticated_user.clone(),
				peer_addr: self.peer_addr,
			})
			.await
			.map_err(|err| {
				error!(
					"Failed to send new connection info to accept loop: {:?}",
					err
				);
				russh::Error::Disconnect
			})?;

		// dropping the handle instead would reject the channel.
		reply.accept().await;
		Ok(())
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
		channel: ChannelId,
		term: &str,
		col_width: u32,
		row_height: u32,
		pix_width: u32,
		pix_height: u32,
		_modes: &[(russh::Pty, u32)],
		session: &mut Session,
	) -> std::result::Result<(), Self::Error> {
		// a reply only goes out when the client asked for one
		session.channel_success(channel)?;
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
		channel: ChannelId,
		session: &mut Session,
	) -> std::result::Result<(), Self::Error> {
		session.channel_success(channel)?;
		if let Some(tx) = &self.from_client_tx {
			tx.send(SshEvent::RequestShell).await.ok();
		}
		Ok(())
	}

	async fn exec_request(
		&mut self,
		channel: ChannelId,
		data: &[u8],
		session: &mut Session,
	) -> std::result::Result<(), Self::Error> {
		session.channel_success(channel)?;
		if let Some(tx) = &self.from_client_tx {
			let command = String::from_utf8_lossy(data);
			tx.send(SshEvent::RequestExec(command.as_ref().into()))
				.await
				.ok();
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
		// Drop the senders so the beet recv loop gets the disconnect signal and
		// the forwarding task knows the client answered its close.
		if self.channel_id == Some(channel) {
			self.from_client_tx.take();
			self.client_closed_tx.take();
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

#[cfg(test)]
#[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
mod tests {
	use super::*;

	/// `BEET_SSH_HOST_KEY` yields a deterministic key across calls, equal to the
	/// baked-in debug key it was generated from.
	#[beet_core::test]
	fn stable_host_key_from_env() {
		let encoded = SshHostKey::DEBUG.to_string();
		// safety: single-threaded test, restored before returning.
		unsafe { env_ext::set_var("BEET_SSH_HOST_KEY", &encoded) }.unwrap();

		let fingerprint =
			|key: PrivateKey| key.public_key().fingerprint(Default::default());
		let expected = fingerprint(parse_host_key(SshHostKey::DEBUG).unwrap());

		fingerprint(host_key(None).unwrap()).xpect_eq(expected);
		fingerprint(host_key(None).unwrap()).xpect_eq(expected);

		// safety: see above.
		unsafe { env_ext::remove_var("BEET_SSH_HOST_KEY") }.unwrap();
	}

	/// REGRESSION: a key rolled as `base64 -w0 <the ssh-keygen file>` was read
	/// as malformed, so a release box served a random fingerprint every start.
	/// Every form a rolled key takes parses to the same key.
	#[beet_core::test]
	fn parses_every_host_key_encoding() {
		let key = parse_host_key(SshHostKey::DEBUG).unwrap();
		let expected = key.public_key().fingerprint(Default::default());
		let pem = key
			.to_openssh(russh::keys::ssh_key::LineEnding::LF)
			.unwrap()
			.to_string();
		let base64_pem = BASE64_STANDARD.encode(&pem);
		// the line breaks `base64` inserts without `-w0`
		let wrapped = base64_pem
			.as_bytes()
			.chunks(76)
			.map(|line| str::from_utf8(line).unwrap())
			.collect::<Vec<_>>()
			.join("\n");

		for value in [
			pem.clone(),
			base64_pem,
			wrapped,
			SshHostKey::DEBUG.to_string(),
		] {
			parse_host_key(&value)
				.unwrap()
				.public_key()
				.fingerprint(Default::default())
				.xpect_eq(expected);
		}
		parse_host_key(&BASE64_STANDARD.encode("not a key")).unwrap_err();
	}

	/// A test [`SecretStore`] holding `0` under every key, read-only.
	#[derive(Clone)]
	struct Fixed(Option<String>);

	impl SecretStoreProvider for Fixed {
		fn box_clone(&self) -> Box<dyn SecretStoreProvider> {
			Box::new(self.clone())
		}
		fn id(&self) -> &'static str { "fixed" }
		fn scope(&self) -> &SecretScope {
			static ROOT: SecretScope = SecretScope::ROOT;
			&ROOT
		}
		fn for_scope(
			&self,
			_scope: &SecretScope,
		) -> Result<Box<dyn SecretStoreProvider>> {
			Ok(self.box_clone())
		}
		fn address(&self, secret: &SecretRef) -> SmolStr {
			format!("fixed#{}", secret.label()).into()
		}
		fn get(
			&self,
			_secret: SecretRef,
		) -> SendBoxedFuture<Result<Option<String>>> {
			let value = self.0.clone();
			Box::pin(async move { value.xok() })
		}
		fn create(
			&self,
			_secret: SecretRef,
			_value: SmolStr,
			_meta: SecretMeta,
		) -> SendBoxedFuture<Result> {
			Box::pin(async { bevybail!("read-only") })
		}
		fn overwrite(
			&self,
			_secret: SecretRef,
			_value: SmolStr,
			_meta: SecretMeta,
		) -> SendBoxedFuture<Result> {
			Box::pin(async { bevybail!("read-only") })
		}
		fn list(&self) -> SendBoxedFuture<Result<Vec<SecretEntry>>> {
			Box::pin(async { bevybail!("read-only") })
		}
		fn delete(
			&self,
			_secrets: Vec<SecretRef>,
		) -> SendBoxedFuture<Result<Vec<SecretRef>>> {
			Box::pin(async { bevybail!("read-only") })
		}
	}

	/// The fingerprint of the key a server presents whose [`SshHostKeyRef`]
	/// names a declaration landing the secret `ssh-host-key` in a [`Fixed`]
	/// store holding `stored`, required or not.
	async fn declared_fingerprint(
		stored: Option<String>,
		required: bool,
	) -> Result<String> {
		let secret = StoredSecret::new(
			SecretStore::new(Fixed(stored)),
			SecretRef::new("ssh-host-key"),
		);
		let secret = match required {
			true => secret.required("deploy the stack"),
			false => secret,
		};
		let mut world = AsyncPlugin.into_world();
		let declaration = world.spawn(secret).id();
		world
			.spawn(SshHostKeyRef(declaration))
			.run_async_then(async |entity| -> Result<String> {
				let declared = SshHostKey::resolve(&entity).await?;
				host_key(declared.as_ref())?
					.public_key()
					.fingerprint(Default::default())
					.to_string()
					.xok()
			})
			.await
	}

	/// A server presents the key its declared secret holds, and a required
	/// secret its store lacks fails the server: a deployed box whose key is
	/// missing from its stack's store never falls back to a key nobody
	/// pinned. An optional one absent is the process default.
	#[beet_core::test]
	async fn serves_the_declared_key_or_fails() {
		let pem = parse_host_key(SshHostKey::DEBUG)
			.unwrap()
			.to_openssh(russh::keys::ssh_key::LineEnding::LF)
			.unwrap()
			.to_string();
		let expected = parse_host_key(SshHostKey::DEBUG)
			.unwrap()
			.public_key()
			.fingerprint(Default::default())
			.to_string();
		declared_fingerprint(Some(BASE64_STANDARD.encode(&pem)), true)
			.await
			.unwrap()
			.xpect_eq(expected.clone());
		declared_fingerprint(None, true)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("fixed")
			.xpect_contains("no secret at fixed#ssh-host-key")
			.xpect_contains("deploy the stack");
		// the debug build's default, as `host_key(None)` answers
		declared_fingerprint(None, false)
			.await
			.unwrap()
			.xpect_eq(expected.clone());
		declared_fingerprint(Some("not a key".into()), true)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("malformed ssh host key");
	}
}
