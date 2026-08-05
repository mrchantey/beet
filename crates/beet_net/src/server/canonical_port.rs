//! The process-global loopback port, the one piece of server state a *client*
//! needs.
//!
//! Deliberately action-free and no_std, unlike the rest of this module: an
//! authority-less [`Request::send`] rewrites to `127.0.0.1:{port}` whether or not
//! this build compiled a server at all, so the registry cannot live behind the
//! `action` gate the server components ride.
use beet_core::prelude::*;

/// The port the canonical server of this process is listening on.
///
/// "Canonical" is the [`HttpServer::canonical`](crate::prelude::HttpServer) flag:
/// a listener sets this the instant it learns its real `local_addr`, so the
/// OS-assigned port of a `port: 0` config resolves here too. A secondary listener
/// clears `canonical` and never claims it.
///
/// Read by anything that needs the process's own origin without being told it: an
/// authority-less [`Request::send`], `beet_ui`'s terminal image fetch, the pdf
/// export's page URL.
pub struct CanonicalPort;

/// `None` until a canonical listener binds.
static CANONICAL_PORT: RwLock<Option<u16>> = RwLock::new(None);

impl CanonicalPort {
	/// The bound port, erroring if no canonical listener has bound yet.
	pub fn get() -> Result<u16> {
		CANONICAL_PORT.read().unwrap().ok_or_else(|| {
			bevyhow!("local port not assigned, is the server running yet?")
		})
	}

	/// Register the bound port, overwriting any existing value. Called from the
	/// bind path the instant a canonical listener learns its real `local_addr`.
	pub fn set(port: u16) { *CANONICAL_PORT.write().unwrap() = Some(port); }
}
