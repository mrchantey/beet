//! The one credential seam a write to a PDS goes through.
//!
//! [`AtprotoAuth`] is shaped for all three providers the protocol allows and
//! implemented for one:
//!
//! - [`AppPassword`]: `createSession` on first use, a bearer token on every
//!   request, `refreshSession` when the server answers `ExpiredToken`.
//! - `OAuthPublic`, designed and unimplemented: PAR, PKCE and DPoP (ES256)
//!   through the browser flow, with a custom url scheme or a same-origin https
//!   redirect. The spec caps a public client's sessions and refresh tokens at
//!   two weeks, so a human re-consents fortnightly.
//! - `OAuthConfidential`, designed and unimplemented, the intended end state:
//!   the same flow with a client key the account's stack holds in its secret
//!   store, sessions unlimited, so an agent or a schedule can write where the
//!   session lives.
//!
//! Both OAuth providers need a client metadata document served at
//! `https://<site>/oauth/client-metadata.json` (a static route), a `p256`
//! dependency for DPoP, and a session store for the refresh token and the DPoP
//! key, which is the stack's secret store. `authorize` then signs a DPoP proof
//! for the exact method and url, and `recover` retries with the server's
//! `dpop_nonce`, so either is a drop-in behind this trait.
use crate::client::provider_send;
use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;

/// How a request to a PDS proves it may write.
pub trait AtprotoAuth: 'static + Send + Sync {
	/// The account this credential writes as.
	fn did(&self) -> &Did;

	/// Attach what this request needs: a bearer token, or under OAuth a DPoP
	/// proof signed for this exact method and url plus the bound token.
	fn authorize(&self, request: Request) -> SendBoxedFuture<Result<Request>>;

	/// The server refused the request: refresh the session (or under OAuth
	/// take the server's DPoP nonce) and say whether it may be retried.
	fn recover(&self, error: &XrpcError) -> SendBoxedFuture<Result<bool>>;
}

/// An app password: a per-application password minted in the account's
/// settings, which cannot change the account itself and can be revoked
/// alone. Read from the env var `env_var` names when the first write needs a
/// session, so a declaration holding one costs nothing until it writes.
#[derive(Clone)]
pub struct AppPassword {
	did: Did,
	env_var: SmolStr,
	session: Arc<RwLock<Option<Session>>>,
}

impl core::fmt::Debug for AppPassword {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("AppPassword")
			.field("did", &self.did)
			.field("env_var", &self.env_var)
			.finish_non_exhaustive()
	}
}

/// A live session: the tokens and the server that issued them.
#[derive(Clone)]
struct Session {
	origin: SmolStr,
	access_jwt: SmolStr,
	refresh_jwt: SmolStr,
}

/// The `createSession` and `refreshSession` output, the parts a session keeps.
#[derive(Deserialize)]
struct SessionResponse {
	#[serde(rename = "accessJwt")]
	access_jwt: SmolStr,
	#[serde(rename = "refreshJwt")]
	refresh_jwt: SmolStr,
}

impl AppPassword {
	/// The credential of `did`, its password in the env var `env_var`.
	pub fn new(did: Did, env_var: impl Into<SmolStr>) -> Self {
		Self {
			did,
			env_var: env_var.into(),
			session: default(),
		}
	}

	fn current(&self) -> Option<Session> {
		self.session
			.read()
			.unwrap_or_else(bevy::platform::sync::PoisonError::into_inner)
			.clone()
	}

	fn store(&self, session: Option<Session>) {
		*self
			.session
			.write()
			.unwrap_or_else(bevy::platform::sync::PoisonError::into_inner) = session;
	}

	/// Exchange the password for a session at `origin`.
	async fn create_session(&self, origin: SmolStr) -> Result<Session> {
		let password = env_ext::var(&self.env_var).map_err(|_| {
			bevyhow!(
				"{} writes with the app password in `{}`, which is not set: \
				 mint one at https://bsky.app/settings/app-passwords and seal it \
				 with `beet secrets/set {} --role=env_var`",
				self.did,
				self.env_var,
				self.env_var
			)
		})?;
		let response = provider_send::send(
			Request::post(format!(
				"{origin}/xrpc/com.atproto.server.createSession"
			))
			.with_json_body(&value!({
				"identifier": (self.did.as_str()),
				"password": password
			}))?,
		)
		.await?;
		Self::session(origin, "com.atproto.server.createSession", response)
			.await
	}

	/// Trade the refresh token for a new pair.
	async fn refresh_session(&self, session: Session) -> Result<Session> {
		let response = provider_send::send(
			Request::post(format!(
				"{}/xrpc/com.atproto.server.refreshSession",
				session.origin
			))
			.with_auth_bearer(&session.refresh_jwt),
		)
		.await?;
		Self::session(
			session.origin,
			"com.atproto.server.refreshSession",
			response,
		)
		.await
	}

	async fn session(
		origin: SmolStr,
		method: &str,
		response: Response,
	) -> Result<Session> {
		if !response.status().is_ok() {
			return Err(XrpcError::read(method, response).await.into());
		}
		let tokens = response.json::<SessionResponse>().await?;
		Session {
			origin,
			access_jwt: tokens.access_jwt,
			refresh_jwt: tokens.refresh_jwt,
		}
		.xok()
	}
}

impl AtprotoAuth for AppPassword {
	fn did(&self) -> &Did { &self.did }

	fn authorize(&self, request: Request) -> SendBoxedFuture<Result<Request>> {
		let this = self.clone();
		provider_send::boxed(async move {
			let session = match this.current() {
				Some(session) => session,
				None => {
					let url = request.url();
					let origin = SmolStr::new(format!(
						"{}://{}",
						url.scheme(),
						url.authority().unwrap_or_default()
					));
					let session = this.create_session(origin).await?;
					this.store(Some(session.clone()));
					session
				}
			};
			request.with_auth_bearer(&session.access_jwt).xok()
		})
	}

	fn recover(&self, error: &XrpcError) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		let error = error.error.clone();
		provider_send::boxed(async move {
			match (error.as_str(), this.current()) {
				("ExpiredToken", Some(session)) => {
					// a refresh that fails starts over with a new session
					let refreshed = this.refresh_session(session).await.ok();
					this.store(refreshed);
					true
				}
				("InvalidToken" | "AuthenticationRequired", Some(_)) => {
					this.store(None);
					true
				}
				_ => false,
			}
			.xok()
		})
	}
}
