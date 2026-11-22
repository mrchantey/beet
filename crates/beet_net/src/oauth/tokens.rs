use beet_core::prelude::*;

/// An OAuth token set: the access token, its refresh token where the server
/// issued one, when the access token lapses, and when the person signed in.
///
/// `signed_in` is the interactive login's instant and survives every refresh
/// ([`refreshed`](Self::refreshed)), so a rule bound to a human act (ie an
/// elevated credential only inside the hour after one) reads the act, never a
/// silent refresh. [`Debug`] prints the instants alone.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthTokens {
	access_token: SmolStr,
	refresh_token: Option<SmolStr>,
	expires_at: Timestamp,
	signed_in: Timestamp,
}

impl core::fmt::Debug for OAuthTokens {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("OAuthTokens")
			.field("refreshable", &self.refresh_token.is_some())
			.field("expires_at", &self.expires_at)
			.field("signed_in", &self.signed_in)
			.finish_non_exhaustive()
	}
}

impl OAuthTokens {
	/// How long before its expiry an access token is refreshed.
	pub const REFRESH_WINDOW: Duration = Duration::from_secs(5 * 60);

	/// The tokens an interactive login at `now` was answered with, the
	/// access token lapsing `expires_in` later.
	pub fn signed_in(
		access_token: impl Into<SmolStr>,
		refresh_token: Option<SmolStr>,
		expires_in: Duration,
		now: Timestamp,
	) -> Self {
		Self {
			access_token: access_token.into(),
			refresh_token,
			expires_at: now + expires_in,
			signed_in: now,
		}
	}

	/// The tokens a refresh at `now` answered, the sign in kept and the
	/// refresh token too where the server issued no new one.
	pub fn refreshed(
		&self,
		access_token: impl Into<SmolStr>,
		refresh_token: Option<SmolStr>,
		expires_in: Duration,
		now: Timestamp,
	) -> Self {
		Self {
			access_token: access_token.into(),
			refresh_token: refresh_token.or_else(|| self.refresh_token.clone()),
			expires_at: now + expires_in,
			signed_in: self.signed_in,
		}
	}

	/// The access token, as secret as a password.
	pub fn access_token(&self) -> &str { &self.access_token }

	/// The refresh token, as secret as a password.
	pub fn refresh_token(&self) -> Option<&str> {
		self.refresh_token.as_deref()
	}

	/// When the access token lapses.
	pub fn expires_at(&self) -> Timestamp { self.expires_at }

	/// When the person signed in.
	pub fn signed_in_at(&self) -> Timestamp { self.signed_in }

	/// Whether the access token is good at `now` with the refresh window to
	/// spare.
	pub fn is_fresh(&self, now: Timestamp) -> bool {
		now + Self::REFRESH_WINDOW < self.expires_at
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// A refresh keeps the sign in and, absent a new one, the refresh token;
	/// neither token is ever printed.
	#[beet_core::test]
	fn a_refresh_keeps_the_sign_in() {
		let signed_in = Timestamp::from_secs(1000);
		let tokens = OAuthTokens::signed_in(
			"access-1",
			Some("refresh-1".into()),
			Duration::from_secs(3600),
			signed_in,
		);
		tokens
			.is_fresh(Timestamp::from_secs(1000 + 3000))
			.xpect_true();
		tokens
			.is_fresh(Timestamp::from_secs(1000 + 3400))
			.xpect_false();
		let refreshed = tokens.refreshed(
			"access-2",
			None,
			Duration::from_secs(3600),
			Timestamp::from_secs(5000),
		);
		refreshed.signed_in_at().xpect_eq(signed_in);
		refreshed.refresh_token().xpect_eq(Some("refresh-1"));
		format!("{refreshed:?}")
			.xnot()
			.xpect_contains("access-2")
			.xnot()
			.xpect_contains("refresh-1");
	}
}
