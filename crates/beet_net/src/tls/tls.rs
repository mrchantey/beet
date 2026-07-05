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
/// Declared in markup like any component, so an entry states its own needs:
/// `<Router {(HttpServer, Tls)}>`.
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
/// Managed platforms (Fargate behind an ALB, lambda behind a gateway)
/// terminate TLS in front of the app; their environments are detected
/// ([`Self::platform_tls_layer`]) and the component goes inert, so the same
/// entry serves plain http there with zero config. `BEET_TLS=on`/`off`
/// overrides the detection either way, eg `off` behind a self-managed
/// reverse proxy.
#[derive(Default, Clone, Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub struct Tls {
	/// PEM certificate chain path, eg a Let's Encrypt `fullchain.pem`. Both
	/// `cert` and `key` set means real certificates; neither means the cached
	/// self-signed dev certificate.
	pub cert: Option<SmolStr>,
	/// PEM private key path, eg a Let's Encrypt `privkey.pem`.
	pub key: Option<SmolStr>,
}

impl Tls {
	/// Whether this config serves TLS here: forced by a `BEET_TLS=on`/`off`
	/// env, otherwise on unless a managed platform terminating TLS in front
	/// of the app is detected ([`Self::platform_tls_layer`]). Deliberately
	/// not keyed on the build profile: the installed cli is a release build
	/// serving local machines, while deployment is an environment.
	pub fn active(&self) -> bool {
		match env_ext::var("BEET_TLS").as_deref().map(str::trim) {
			Ok("1") | Ok("true") | Ok("on") => true,
			Ok("0") | Ok("false") | Ok("off") => false,
			_ => !Self::platform_tls_layer(),
		}
	}

	/// Whether a managed platform that terminates TLS in front of the app is
	/// detected: lambda (`AWS_LAMBDA_FUNCTION_NAME`) or ECS/Fargate (the
	/// `ECS_CONTAINER_METADATA_URI*` / `AWS_EXECUTION_ENV` markers). A setup
	/// fronting its own TLS some other way (a reverse proxy on a VPS) sets
	/// `BEET_TLS=off` instead.
	pub fn platform_tls_layer() -> bool {
		[
			"AWS_LAMBDA_FUNCTION_NAME",
			"ECS_CONTAINER_METADATA_URI_V4",
			"ECS_CONTAINER_METADATA_URI",
			"AWS_EXECUTION_ENV",
		]
		.iter()
		.any(|key| env_ext::var(key).is_ok())
	}

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
