use base64::Engine;
use beet_core::prelude::*;
use sha2::Digest;
use sha2::Sha256;

/// A PKCE pair (RFC 7636, method `S256`): the verifier this client keeps and
/// the challenge the authorization url carries, so only the client that
/// started a login can redeem its code. [`Debug`] prints the challenge alone.
#[derive(Clone, PartialEq, Eq)]
pub struct Pkce {
	verifier: SmolStr,
	challenge: SmolStr,
}

impl core::fmt::Debug for Pkce {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Pkce")
			.field("challenge", &self.challenge)
			.finish_non_exhaustive()
	}
}

impl Pkce {
	/// The challenge method every pair here uses.
	pub const METHOD: &'static str = "S256";

	/// A fresh pair: 32 random bytes as the verifier's 43 characters.
	pub fn new() -> Self { Self::from_verifier(OAuthState::random_token()) }

	/// The pair of `verifier`, its challenge derived.
	pub fn from_verifier(verifier: impl Into<SmolStr>) -> Self {
		let verifier = verifier.into();
		let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
			.encode(Sha256::digest(verifier.as_bytes()));
		Self {
			verifier,
			challenge: challenge.into(),
		}
	}

	/// The verifier, sent only with the token request.
	pub fn verifier(&self) -> &str { &self.verifier }

	/// The challenge, sent in the authorization url.
	pub fn challenge(&self) -> &str { &self.challenge }
}

/// The value an authorization redirect must hand back unchanged, binding the
/// answer to the request that asked for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthState(SmolStr);

impl OAuthState {
	/// A fresh state.
	pub fn new() -> Self { Self(Self::random_token()) }

	/// The value.
	pub fn as_str(&self) -> &str { &self.0 }

	/// Refuse a redirect whose state is not this one: a forged or replayed
	/// answer.
	pub fn check(&self, returned: Option<&str>) -> Result {
		match returned == Some(self.0.as_str()) {
			true => OK,
			false => bevybail!(
				"the authorization answer carried another login's state, so it \
				was refused: start the login again"
			),
		}
	}

	/// 32 random bytes, base64url without padding.
	pub(crate) fn random_token() -> SmolStr {
		let bytes = RandomSource::default().random::<[u8; 32]>();
		base64::engine::general_purpose::URL_SAFE_NO_PAD
			.encode(bytes)
			.into()
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// RFC 7636's own example pair, and the verifier never printed.
	#[beet_core::test]
	fn derives_the_published_challenge() {
		let pkce =
			Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
		pkce.challenge()
			.xpect_eq("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
		format!("{pkce:?}").xnot().xpect_contains("dBjftJeZ4CVP");
		Pkce::new().verifier().len().xpect_eq(43);
	}

	/// A state checks against itself alone.
	#[beet_core::test]
	fn checks_the_state() {
		let state = OAuthState::new();
		state.check(Some(state.as_str())).unwrap();
		state.check(Some("forged")).unwrap_err();
		state.check(None).unwrap_err();
	}
}
