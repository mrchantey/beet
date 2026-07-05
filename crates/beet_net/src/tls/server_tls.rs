//! The resolved rustls acceptor a TLS listener runs.
use crate::prelude::*;
use beet_core::prelude::*;
use futures_rustls::TlsAcceptor;
use futures_rustls::rustls;
use std::sync::Arc;

/// A listener's TLS acceptor, built from its entity's [`Tls`] component (see
/// [`MaybeTls::resolve`]): the dev certificate by default, provided PEM files
/// when both paths are set.
#[derive(Clone)]
pub struct ServerTls {
	acceptor: TlsAcceptor,
	provided: bool,
}

impl ServerTls {
	/// Build the acceptor from a [`Tls`] config. Uses an explicit `ring`
	/// provider, matching the wss client (`default_rustls_client_config`), to
	/// avoid the multi-provider runtime panic.
	pub fn from_component(tls: &Tls) -> Result<Self> {
		let (certs, key) = match (&tls.cert, &tls.key) {
			(Some(cert), Some(key)) => DevCert::parse_pem(
				&fs_ext::read(cert.as_str())?,
				&fs_ext::read(key.as_str())?,
			)?,
			(None, None) => DevCert::load_or_generate()?,
			_ => bevybail!(
				"Tls needs both `cert` and `key` paths, or neither (the \
				generated dev certificate)"
			),
		};
		let provider = rustls::crypto::ring::default_provider();
		let config =
			rustls::ServerConfig::builder_with_provider(Arc::new(provider))
				.with_safe_default_protocol_versions()?
				.with_no_client_auth()
				.with_single_cert(certs, key)?;
		Self {
			acceptor: TlsAcceptor::from(Arc::new(config)),
			provided: tls.provided(),
		}
		.xok()
	}

	/// Whether real (provided) certificates back this acceptor, in which case
	/// a socket listener rejects remote plaintext rather than serving it.
	pub fn provided(&self) -> bool { self.provided }

	/// Perform the server-side TLS handshake.
	pub async fn accept<S>(
		&self,
		stream: S,
	) -> Result<futures_rustls::server::TlsStream<S>>
	where
		S: futures_lite::AsyncRead + futures_lite::AsyncWrite + Unpin,
	{
		self.acceptor.accept(stream).await?.xok()
	}
}
