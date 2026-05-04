use beet_core::prelude::*;
use bytes::Bytes;

/// SSH data payload sent between client and server.
#[derive(Debug, Clone)]
pub enum SshData {
	/// Raw binary data.
	Bytes(Bytes),
	/// Process exit code.
	Exit(u32),
}

impl SshData {
	/// Create a bytes payload.
	pub fn bytes(data: impl Into<Bytes>) -> Self { Self::Bytes(data.into()) }

	/// Create a UTF-8 text payload encoded as bytes.
	pub fn text(text: impl Into<String>) -> Self {
		Self::Bytes(Bytes::from(text.into().into_bytes()))
	}

	/// Returns the inner byte slice if this is a [`SshData::Bytes`] variant.
	pub fn as_bytes(&self) -> Option<&[u8]> {
		match self {
			Self::Bytes(b) => Some(b.as_ref()),
			_ => None,
		}
	}

	/// Returns the inner string slice if this is a [`SshData::Bytes`] variant containing valid UTF-8.
	pub fn as_str(&self) -> Option<&str> {
		self.as_bytes().and_then(|b| std::str::from_utf8(b).ok())
	}
}

/// A message to be sent to the SSH peer.
#[derive(Debug, Clone, Deref, EntityTargetEvent)]
#[event(auto_propagate)]
pub struct SshDataSend(pub SshData);

impl SshDataSend {
	/// Consumes self and returns the inner [`SshData`].
	pub fn take(self) -> SshData { self.0 }
	/// Returns a reference to the inner [`SshData`].
	pub fn inner(&self) -> &SshData { &self.0 }
}

/// A message received from the SSH peer.
#[derive(Debug, Clone, Deref, EntityTargetEvent)]
#[event(auto_propagate)]
pub struct SshDataRecv(pub SshData);

impl SshDataRecv {
	/// Consumes self and returns the inner [`SshData`].
	pub fn take(self) -> SshData { self.0 }
	/// Returns a reference to the inner [`SshData`].
	pub fn inner(&self) -> &SshData { &self.0 }
}
