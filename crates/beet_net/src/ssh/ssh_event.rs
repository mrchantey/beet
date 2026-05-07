use beet_core::prelude::*;
use bytes::Bytes;

/// SSH event payload exchanged between client and server.
///
/// A single enum covering the full lifecycle: connection, data transfer,
/// terminal control, and disconnection.
#[derive(Debug, Clone)]
pub enum SshEvent {
	/// Connection established.
	///
	/// Received on the server when a client opens a session.
	/// Received on the client when the session is ready.
	Connect,
	/// Raw binary data payload.
	Data(Bytes),
	/// Client requests a pseudo-terminal from the server.
	RequestPty(RequestPty),
	/// Terminal window resize notification.
	Resize(SshWindowSize),
	/// Client requests a shell session.
	RequestShell,
	/// Client requests X11 forwarding.
	RequestX11(RequestX11),
	/// Session closed, optionally with an exit code and reason.
	///
	/// **Send** this to close the connection.
	/// **Receive** this when the peer closes.
	Close(Option<SshCloseFrame>),
}

impl SshEvent {
	/// Create a [`SshEvent::Data`] payload from a UTF-8 string.
	pub fn text(text: impl Into<String>) -> Self {
		Self::Data(Bytes::from(text.into().into_bytes()))
	}

	/// Create a [`SshEvent::Data`] payload from raw bytes.
	pub fn bytes(data: impl Into<Bytes>) -> Self { Self::Data(data.into()) }

	/// Returns the inner byte slice if this is a [`SshEvent::Data`] variant.
	pub fn as_bytes(&self) -> Option<&[u8]> {
		match self {
			Self::Data(b) => Some(b.as_ref()),
			_ => None,
		}
	}

	/// Returns the inner string slice if this is a [`SshEvent::Data`] variant containing valid UTF-8.
	pub fn as_str(&self) -> Option<&str> {
		self.as_bytes().and_then(|b| std::str::from_utf8(b).ok())
	}
}

/// A message to send to the SSH peer.
///
/// Trigger this on a connection entity to forward data to the peer.
/// Listen for outgoing events by adding a global observer via [`App::add_observer`].
#[derive(Debug, Clone, Deref, EntityTargetEvent)]
pub struct SshSend(pub SshEvent);

impl SshSend {
	/// Consumes self and returns the inner [`SshEvent`].
	pub fn take(self) -> SshEvent { self.0 }
	/// Returns a reference to the inner [`SshEvent`].
	pub fn inner(&self) -> &SshEvent { &self.0 }
}

/// A message received from the SSH peer.
///
/// Triggered on the connection entity when data arrives from the peer.
/// Listen for incoming events by adding a global observer via [`App::add_observer`].
/// The server entity (if any) can be obtained via `ev.target()`'s parent.
#[derive(Debug, Clone, Deref, EntityTargetEvent)]
pub struct SshRecv(pub SshEvent);

impl SshRecv {
	/// Consumes self and returns the inner [`SshEvent`].
	pub fn take(self) -> SshEvent { self.0 }
	/// Returns a reference to the inner [`SshEvent`].
	pub fn inner(&self) -> &SshEvent { &self.0 }
}

/// Close frame attached to [`SshEvent::Close`].
#[derive(Debug, Clone)]
pub struct SshCloseFrame {
	/// SSH process exit code (0 = success).
	pub code: u32,
	/// Human-readable reason for the closure.
	pub reason: SmolStr,
}

impl SshCloseFrame {
	/// Returns `Ok(())` for code 0, or a [`BevyError`] for non-zero codes.
	pub fn into_result(self) -> Result {
		if self.code == 0 {
			Ok(())
		} else {
			Err(bevyhow!(
				"SSH session closed: {} (code {})",
				self.reason,
				self.code
			))
		}
	}
}

impl From<SshCloseFrame> for Result {
	fn from(frame: SshCloseFrame) -> Self { frame.into_result() }
}

/// Pseudo-terminal request parameters, sent by the client to the server.
#[derive(Debug, Clone)]
pub struct RequestPty {
	/// Terminal type string, e.g. `"xterm-256color"`.
	pub terminal: SmolStr,
	/// Terminal window dimensions.
	pub window: SshWindowSize,
	/// Terminal mode codes as (code, value) pairs per RFC 4254 §8.
	pub terminal_modes: Vec<(u8, u32)>,
}

/// Terminal window dimensions.
#[derive(Debug, Clone, Copy)]
pub struct SshWindowSize {
	/// Width and height in character cells (columns × rows).
	pub cells: UVec2,
	/// Width and height in pixels.
	pub pixels: UVec2,
}

/// X11 forwarding request parameters.
#[derive(Debug, Clone)]
pub struct RequestX11 {
	/// X11 authentication protocol name.
	pub auth_protocol: SmolStr,
	/// X11 authentication cookie.
	pub auth_cookie: SmolStr,
	/// X11 screen number.
	pub screen: u32,
}

/// Per-connection info inserted on each connection entity when a client opens a session.
#[derive(Debug, Clone, Component)]
pub struct SshPeerInfo {
	/// The username supplied during authentication, if any.
	pub username: Option<SmolStr>,
}
