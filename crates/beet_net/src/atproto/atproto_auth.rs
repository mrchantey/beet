//! An atproto account's app password, one [`RequestAuth`] a write to a PDS
//! goes through; the OAuth credentials designed beside it are on that seam's
//! own docs.
use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;

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

	/// The account this credential writes as.
	pub fn did(&self) -> &Did { &self.did }

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
		let response = Request::post(format!(
			"{origin}/xrpc/com.atproto.server.createSession"
		))
		.with_json_body(&value!({
			"identifier": (self.did.as_str()),
			"password": password
		}))?
		.send()
		.await?;
		Self::session(origin, "com.atproto.server.createSession", response)
			.await
	}

	/// Trade the refresh token for a new pair.
	async fn refresh_session(&self, session: Session) -> Result<Session> {
		let response = Request::post(format!(
			"{}/xrpc/com.atproto.server.refreshSession",
			session.origin
		))
		.with_auth_bearer(&session.refresh_jwt)
		.send()
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

impl RequestAuth for AppPassword {
	fn authorize(&self, request: Request) -> SendBoxedFuture<Result<Request>> {
		let this = self.clone();
		Box::pin(async move {
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

	fn recover(&self, failure: &AuthFailure) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		let error = failure.code.clone();
		Box::pin(async move {
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
