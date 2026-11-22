//! The reusable pieces of an OAuth 2.0 client.
//!
//! - [`Pkce`]: the S256 code verifier and challenge, and [`OAuthState`], the
//!   value a redirect must hand back unchanged.
//! - [`LoopbackRedirect`]: a one-shot receiver on beet's own [`HttpServer`]
//!   at `http://127.0.0.1:<port>/oauth/callback`, answering the browser a
//!   page saying it may close.
//! - [`OAuthBrowser`]: the authorization url handed to the operator as a
//!   notice ([`OperatorPrompt`]), opened in a browser where the platform can.
//! - [`DeviceGrant`]: the device authorization poll (RFC 8628), respecting
//!   `interval` and `slow_down`, for a machine with no browser to redirect.
//! - [`OAuthTokens`]: an access token, its refresh token and when each was
//!   issued and lapses.
//!
//! A provider's own endpoints and wire shapes are its client's business (ie
//! IAM Identity Center's, `IdentityCenterSession`). The atproto OAuth
//! credentials designed on the [`RequestAuth`] docs attach here as well: PKCE,
//! state, the loopback receiver and the token set are theirs as written, and
//! they add a PAR request and DPoP proofs beside them, plus a client metadata
//! document a static route serves.
mod browser;
mod device_grant;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod loopback;
mod pkce;
mod tokens;
pub use browser::*;
pub use device_grant::*;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub use loopback::*;
pub use pkce::*;
pub use tokens::*;

/// A world whose async tasks run on a thread of its own, the shape a verb's
/// caller world takes, for a test of a login that spawns its receiver there.
#[cfg(all(test, feature = "server", not(target_arch = "wasm32")))]
pub(crate) fn threaded_world() -> beet_core::prelude::AsyncWorld {
	use crate::prelude::*;
	use beet_core::prelude::*;
	let (send, recv) = std::sync::mpsc::channel();
	std::thread::spawn(move || {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		send.send(app.world().resource::<AsyncWorld>().clone()).ok();
		app.run();
	});
	recv.recv().unwrap()
}
