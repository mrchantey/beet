//! The [`Tls`] config component and the [`MaybeTls`] state a listener
//! resolves it into.
#[cfg(all(feature = "secure", not(target_arch = "wasm32")))]
use crate::prelude::*;
use beet_core::prelude::*;

/// Serve TLS on this entity's server ([`HttpServer`](crate::prelude::HttpServer)
/// or [`SocketServer`](crate::sockets::SocketServer)).
///
/// With no paths a self-signed dev certificate is generated and cached (see
/// `DevCert`), granting browsers a secure context on LAN origins after a
/// one-time warning click-through. Provide `cert`/`key` PEM paths for real
/// certificates on a self-hosted deployment. Requires the `secure` feature;
/// without it the component is registered but serving stays plaintext (with a
/// warning).
///
/// The `--secure` boot flag inserts this component (with `dev_only: false`)
/// on every server the boot selects, so an entry needs no markup change to
/// serve TLS for a session.
///
/// ## Plaintext peers
/// A TLS listener still classifies each connection by its first bytes:
/// - http: plaintext from loopback is served as-is (localhost is already a
///   secure context, and the reload watcher connects there); plaintext from a
///   remote peer gets a `307` redirect to https.
/// - socket: plaintext websockets stay served (native and embedded clients
///   have no secure-context requirement, and the dev cert grants no transport
///   security anyway); with provided (real) certificates remote plaintext is
///   rejected instead.
///
/// ## Deployed
/// Platform deployments (Fargate behind an ALB, lambda behind a gateway)
/// terminate TLS in front of the app, so `dev_only` (default `true`) makes
/// the component inert in release builds: the same entry serves plain http
/// there with zero config. Set `dev_only: false` for a self-hosted release
/// with real certificates.
#[derive(Clone, Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub struct Tls {
	/// PEM certificate chain path, eg a Let's Encrypt `fullchain.pem`. Both
	/// `cert` and `key` set means real certificates; neither means the cached
	/// self-signed dev certificate.
	pub cert: Option<SmolStr>,
	/// PEM private key path, eg a Let's Encrypt `privkey.pem`.
	pub key: Option<SmolStr>,
	/// Only serve TLS in debug builds (default `true`): a release build, eg
	/// deployed behind a platform TLS layer, serves plaintext untouched.
	pub dev_only: bool,
}

impl Default for Tls {
	fn default() -> Self {
		Self {
			cert: None,
			key: None,
			dev_only: true,
		}
	}
}

impl Tls {
	/// Serve TLS in release builds too, ie a self-hosted deployment with real
	/// certificates, or the explicit `--secure` boot flag.
	pub fn always(mut self) -> Self {
		self.dev_only = false;
		self
	}

	/// Whether this config serves TLS in the current build profile.
	pub fn active(&self) -> bool { !self.dev_only || cfg!(debug_assertions) }

	/// Whether real certificates are provided (both paths set).
	pub fn provided(&self) -> bool { self.cert.is_some() && self.key.is_some() }
}

/// The resolved TLS state a native listener carries into its accept loop:
/// inert unless the entity holds an [`active`](Tls::active) [`Tls`] and the
/// `secure` feature is compiled in.
#[derive(Default, Clone)]
pub struct MaybeTls {
	#[cfg(all(feature = "secure", not(target_arch = "wasm32")))]
	inner: Option<ServerTls>,
}

impl MaybeTls {
	/// Read the entity's [`Tls`] and build the acceptor. Inert when the
	/// component is absent or inactive; errors when cert loading/generation
	/// fails, so a misconfigured server fails its boot loudly.
	pub async fn resolve(entity: &AsyncEntity) -> Result<Self> {
		let tls = entity
			.with(|entity| entity.get::<Tls>().cloned())
			.await?
			.filter(|tls| tls.active());
		#[cfg(all(feature = "secure", not(target_arch = "wasm32")))]
		{
			tls.map(|tls| ServerTls::from_component(&tls))
				.transpose()?
				.xmap(|inner| Self { inner })
				.xok()
		}
		#[cfg(not(all(feature = "secure", not(target_arch = "wasm32"))))]
		{
			if tls.is_some() {
				warn!(
					"Tls component is present but the `secure` feature is not \
					compiled in: serving plaintext. Rebuild with `--features secure`."
				);
			}
			Ok(Self::default())
		}
	}

	/// Whether the listener serves TLS.
	pub fn is_active(&self) -> bool {
		#[cfg(all(feature = "secure", not(target_arch = "wasm32")))]
		return self.inner.is_some();
		#[cfg(not(all(feature = "secure", not(target_arch = "wasm32"))))]
		return false;
	}

	/// Whether real (provided) certificates back the acceptor, in which case
	/// a socket listener rejects remote plaintext rather than serving it.
	pub fn provided(&self) -> bool {
		#[cfg(all(feature = "secure", not(target_arch = "wasm32")))]
		return self
			.inner
			.as_ref()
			.is_some_and(|server_tls| server_tls.provided());
		#[cfg(not(all(feature = "secure", not(target_arch = "wasm32"))))]
		return false;
	}

	/// The acceptor, when serving TLS.
	#[cfg(all(feature = "secure", not(target_arch = "wasm32")))]
	pub fn get(&self) -> Option<&ServerTls> { self.inner.as_ref() }

	/// `https`/`http` for logs and bind messages.
	pub fn http_scheme(&self) -> &'static str {
		match self.is_active() {
			true => "https",
			false => "http",
		}
	}

	/// `wss`/`ws` for logs and bind messages.
	pub fn ws_scheme(&self) -> &'static str {
		match self.is_active() {
			true => "wss",
			false => "ws",
		}
	}
}
