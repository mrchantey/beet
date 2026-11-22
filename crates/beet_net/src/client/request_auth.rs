//! The one seam a request proves who it is through.
//!
//! [`RequestAuth`] is two calls: [`authorize`](RequestAuth::authorize)
//! attaches whatever this exact request needs, and
//! [`recover`](RequestAuth::recover) reads a refusal and says whether to try
//! again. A client holds one as an `Arc<dyn RequestAuth>` and calls it around
//! every send, so a credential swaps without the client knowing its kind.
//!
//! ## Implementations
//!
//! - `AwsAuth` (the `aws` feature): Signature Version 4 as a credential
//!   source's current credential. Recovery refreshes a lapsed source, and on
//!   a clock skew refusal corrects the process's clock offset from the
//!   response `Date`.
//! - `AppPassword` (the `atproto` feature): an atproto session, created on
//!   first use and sent as a bearer token. Recovery refreshes it on
//!   `ExpiredToken` and starts over on `InvalidToken`.
//! - [`NoAuth`]: nothing attached, ie an anonymous read of a public bucket.
//!
//! ## atproto OAuth
//!
//! The protocol's other two credentials are designed against this seam and
//! not yet built:
//!
//! - `OAuthPublic`: PAR, PKCE and DPoP (ES256) through the browser flow, with
//!   a custom url scheme or a same-origin https redirect. The spec caps a
//!   public client's sessions and refresh tokens at two weeks, so a human
//!   re-consents fortnightly.
//! - `OAuthConfidential`, the intended end state: the same flow with a client
//!   key the account's stack holds in its secret store, sessions unlimited,
//!   so an agent or a schedule can write where the session lives.
//!
//! Both reuse the `oauth` module's primitives: PKCE, state, the loopback
//! redirect receiver, the authorization url handed to the operator as a
//! notice, and the refreshing token set. They add their own pieces: a PAR
//! request, a client metadata document served at
//! `https://<site>/oauth/client-metadata.json` (a static route), a `p256`
//! dependency for DPoP, and the stack's secret store holding the refresh
//! token and the DPoP key. `authorize` then signs a DPoP proof for the exact
//! method and url, and `recover` retries with the `DPoP-Nonce` the
//! [`AuthFailure`]'s headers carry, so either is a drop-in behind this trait.
//!
//! ## Why a handle and not a declared provider
//!
//! No scene declares an auth by itself: it is always part of something that
//! is declared, an `<AtprotoAccount/>`'s secret, a store's credential, a
//! stack's identity, whose own attach builds it. So it is a plain
//! `Arc<dyn RequestAuth>` held by the client that sends, rather than a
//! component with a reflect declaration, which would park a credential in
//! the world with no consumer to read it there.
use crate::prelude::*;
use beet_core::prelude::*;

/// How a request proves who it is: attach a credential, then read a refusal.
pub trait RequestAuth: 'static + Send + Sync {
	/// Attach what this request needs: a signature, a bearer token, a proof
	/// for this exact method and url.
	fn authorize(&self, request: Request) -> SendBoxedFuture<Result<Request>>;

	/// The server refused `failure`'s request: refresh, correct, or give up.
	/// `true` means the request may be retried, once.
	fn recover(&self, failure: &AuthFailure) -> SendBoxedFuture<Result<bool>> {
		let _ = failure;
		Box::pin(async { Ok(false) })
	}
}

/// What a refusal says, which is what every recovery reads: the status, the
/// protocol's error code and message, and the response headers (a `Date` to
/// correct a skewed clock by, a `DPoP-Nonce` to retry with).
#[derive(Debug, Clone)]
pub struct AuthFailure {
	/// The http status.
	pub status: StatusCode,
	/// The protocol's error code, ie `ExpiredToken`, empty when it named none.
	pub code: SmolStr,
	/// The server's explanation, empty when it gave none.
	pub message: String,
	/// The response headers.
	pub headers: HeaderMap,
}

impl AuthFailure {
	/// The first value of the header `name` carried.
	pub fn header(&self, name: &str) -> Option<&str> {
		self.headers.first_raw(name)
	}
}

/// No credential: the request is sent as it is, ie an anonymous read.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoAuth;

impl RequestAuth for NoAuth {
	fn authorize(&self, request: Request) -> SendBoxedFuture<Result<Request>> {
		Box::pin(async move { Ok(request) })
	}
}
