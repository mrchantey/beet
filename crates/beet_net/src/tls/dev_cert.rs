//! The generated + cached self-signed dev certificate backing
//! [`Tls`](crate::prelude::Tls) when no cert paths are provided.
use beet_core::prelude::*;
use futures_rustls::rustls;
use rustls_pki_types::CertificateDer;
use rustls_pki_types::PrivateKeyDer;
use rustls_pki_types::pem::PemObject;
use std::path::PathBuf;
use std::sync::Arc;

/// The self-signed dev certificate: generated with `rcgen` on first use and
/// cached as PEM, so browser exceptions survive restarts (Firefox pins an
/// exception to the certificate itself, a fresh cert per run would re-prompt
/// every boot). Regenerated when the subject-alt-names change, eg the machine
/// moved networks.
///
/// The cert is deliberately untrusted: browsers show a one-time warning to
/// click through per origin. It grants a secure context, not transport
/// security; committing a cert to the repo would grant neither more.
pub struct DevCert;

impl DevCert {
	/// The cache directory: `$BEET_TLS_DIR`, falling back to `target/tls`
	/// under the workspace root.
	pub fn dir() -> PathBuf {
		env_ext::var("BEET_TLS_DIR")
			.map(PathBuf::from)
			.unwrap_or_else(|_| fs_ext::workspace_root().join("target/tls"))
	}

	/// The cached PEM certificate. Also the file to import on a device that
	/// should trust the cert outright instead of clicking through warnings.
	pub fn cert_path() -> PathBuf { Self::dir().join("cert.pem") }

	/// The cached PEM private key.
	pub fn key_path() -> PathBuf { Self::dir().join("key.pem") }

	/// The subject-alt-names the cached cert was generated for, one per line;
	/// a mismatch with [`Self::subject_alt_names`] triggers regeneration.
	fn sans_path(dir: &PathBuf) -> PathBuf { dir.join("sans.txt") }

	/// The names the cert answers for: localhost, the loopback addresses and
	/// the machine's primary LAN address (the one a phone dials).
	pub fn subject_alt_names() -> Vec<String> {
		let mut sans =
			vec!["localhost".into(), "127.0.0.1".into(), "::1".into()];
		if let Some(ip) = Self::lan_ip() {
			sans.push(ip.to_string());
		}
		sans
	}

	/// The primary outbound address: `connect` a UDP socket (no packets are
	/// sent) and read the OS-chosen source address.
	fn lan_ip() -> Option<core::net::IpAddr> {
		let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
		socket.connect("8.8.8.8:80").ok()?;
		socket.local_addr().ok()?.ip().xmap(Some)
	}

	/// Load the cached cert/key pair, generating and caching a fresh one when
	/// missing, unparsable, or the subject-alt-names changed.
	pub fn load_or_generate()
	-> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
		Self::load_or_generate_in(&Self::dir())
	}

	/// [`Self::load_or_generate`] against an explicit cache directory (tests).
	pub(crate) fn load_or_generate_in(
		dir: &PathBuf,
	) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
		// serialize check-then-generate so concurrent listeners (two servers
		// booting, parallel tests) never interleave a half-written pair
		static GENERATE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
		let _guard = GENERATE_LOCK
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		let sans = Self::subject_alt_names();
		let sans_file = sans.join("\n");
		let cert_path = dir.join("cert.pem");
		let key_path = dir.join("key.pem");

		// reuse the cached pair while the names still match
		if fs_ext::read_to_string(Self::sans_path(dir))
			.map(|cached| cached == sans_file)
			.unwrap_or(false)
			&& let (Ok(cert_pem), Ok(key_pem)) =
				(fs_ext::read(&cert_path), fs_ext::read(&key_path))
			&& let Ok(pair) = Self::parse_pem(&cert_pem, &key_pem)
		{
			return Ok(pair);
		}

		let generated = rcgen::generate_simple_self_signed(sans.clone())?;
		fs_ext::write(&cert_path, generated.cert.pem())?;
		fs_ext::write(&key_path, generated.signing_key.serialize_pem())?;
		fs_ext::write(Self::sans_path(dir), &sans_file)?;
		info!(
			"generated self-signed dev certificate for [{}] in {}",
			sans.join(", "),
			dir.display()
		);
		(
			vec![CertificateDer::from(generated.cert.der().to_vec())],
			PrivateKeyDer::Pkcs8(generated.signing_key.serialize_der().into()),
		)
			.xok()
	}

	/// Parse a PEM certificate chain and private key, ie the cached dev cert
	/// or user-provided files (a Let's Encrypt `fullchain.pem`/`privkey.pem`).
	pub fn parse_pem(
		cert_pem: &[u8],
		key_pem: &[u8],
	) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
		let certs = CertificateDer::pem_slice_iter(cert_pem)
			.collect::<Result<Vec<_>, _>>()?;
		if certs.is_empty() {
			bevybail!("no certificates found in PEM");
		}
		let key = PrivateKeyDer::from_pem_slice(key_pem)?;
		(certs, key).xok()
	}

	/// A [`rustls::ClientConfig`] trusting exactly the cached dev cert: how a
	/// native client (or test) connects to a locally served `https`/`wss`.
	pub fn client_config() -> Result<rustls::ClientConfig> {
		let cert_pem = fs_ext::read(Self::cert_path())?;
		let mut root_store = rustls::RootCertStore::empty();
		for cert in CertificateDer::pem_slice_iter(&cert_pem) {
			root_store.add(cert?)?;
		}
		let provider = rustls::crypto::ring::default_provider();
		rustls::ClientConfig::builder_with_provider(Arc::new(provider))
			.with_safe_default_protocol_versions()?
			.with_root_certificates(root_store)
			.with_no_client_auth()
			.xok()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	fn caches_and_regenerates() {
		let dir = fs_ext::workspace_root().join("target/tls-test");
		fs_ext::remove(&dir).ok();

		// first call generates
		let (certs, _key) = DevCert::load_or_generate_in(&dir).unwrap();
		certs.len().xpect_greater_than(0usize);

		// second call reuses the cached pair byte-for-byte
		let cert_pem = fs_ext::read(dir.join("cert.pem")).unwrap();
		let (again, _key) = DevCert::load_or_generate_in(&dir).unwrap();
		again.xpect_eq(certs);

		// a SAN change regenerates
		fs_ext::write(dir.join("sans.txt"), "stale.example").unwrap();
		DevCert::load_or_generate_in(&dir).unwrap();
		fs_ext::read(dir.join("cert.pem"))
			.unwrap()
			.xpect_not_eq(cert_pem);
	}
}
