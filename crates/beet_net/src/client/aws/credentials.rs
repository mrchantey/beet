use beet_core::prelude::*;

/// The AWS identity a request is signed as: an access key pair and, for a
/// temporary credential, its session token and the instant it lapses.
/// [`Debug`] prints the key id alone.
///
/// Where one comes from is an [`AwsCredentialSource`]; how it signs is
/// [`SigV4`].
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AwsCredentials {
	access_key_id: SmolStr,
	secret_access_key: SmolStr,
	session_token: Option<SmolStr>,
	expiry: Option<Timestamp>,
}

impl core::fmt::Debug for AwsCredentials {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("AwsCredentials")
			.field("access_key_id", &self.access_key_id)
			.field("expiry", &self.expiry)
			.finish_non_exhaustive()
	}
}

impl AwsCredentials {
	/// The variable holding the key id.
	pub const ACCESS_KEY_ID: &'static str = "AWS_ACCESS_KEY_ID";
	/// The variable holding the secret half.
	pub const SECRET_ACCESS_KEY: &'static str = "AWS_SECRET_ACCESS_KEY";
	/// The variable holding a temporary credential's session token.
	pub const SESSION_TOKEN: &'static str = "AWS_SESSION_TOKEN";

	/// The long-lived pair `access_key_id` and `secret_access_key`.
	pub fn new(
		access_key_id: impl Into<SmolStr>,
		secret_access_key: impl Into<SmolStr>,
	) -> Self {
		Self {
			access_key_id: access_key_id.into(),
			secret_access_key: secret_access_key.into(),
			session_token: None,
			expiry: None,
		}
	}

	/// The same pair as a temporary credential carrying `token`.
	pub fn with_session_token(mut self, token: impl Into<SmolStr>) -> Self {
		self.session_token = Some(token.into());
		self
	}

	/// The same credential lapsing at `expiry`.
	pub fn with_expiry(mut self, expiry: Timestamp) -> Self {
		self.expiry = Some(expiry);
		self
	}

	/// The key id, the one part safe to print.
	pub fn access_key_id(&self) -> &str { &self.access_key_id }

	/// The secret half. Never logged or printed: it goes into a signature,
	/// or into the environment of a child that signs its own requests.
	pub fn secret_access_key(&self) -> &str { &self.secret_access_key }

	/// A temporary credential's session token, as secret as the key.
	pub fn session_token(&self) -> Option<&str> {
		self.session_token.as_deref()
	}

	/// When a temporary credential lapses, `None` for a long-lived pair.
	pub fn expiry(&self) -> Option<Timestamp> { self.expiry }

	/// The credential as the variables a child process signing its own
	/// requests reads (`tofu`, `docker`, the `aws` cli): the pair, and the
	/// session token where there is one.
	pub fn env_vars(&self) -> Vec<(SmolStr, SmolStr)> {
		let mut vars = vec![
			(Self::ACCESS_KEY_ID.into(), self.access_key_id.clone()),
			(
				Self::SECRET_ACCESS_KEY.into(),
				self.secret_access_key.clone(),
			),
		];
		if let Some(token) = &self.session_token {
			vars.push((Self::SESSION_TOKEN.into(), token.clone()));
		}
		vars
	}

	/// The environment's pair, `None` unless both halves are set.
	pub fn from_env() -> Option<Self> {
		let access_key_id = env_ext::var(Self::ACCESS_KEY_ID).ok()?;
		let secret_access_key = env_ext::var(Self::SECRET_ACCESS_KEY).ok()?;
		let credentials = Self::new(access_key_id, secret_access_key);
		match env_ext::var(Self::SESSION_TOKEN) {
			Ok(token) if !token.is_empty() => {
				credentials.with_session_token(token)
			}
			_ => credentials,
		}
		.xsome()
	}

	/// `profile`'s pair in a shared credentials file's text, `None` when the
	/// file lacks it.
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub(crate) fn parse_profile(text: &str, profile: &str) -> Option<Self> {
		let mut in_profile = false;
		let (mut access_key_id, mut secret_access_key, mut session_token) =
			(None, None, None);
		for line in text.lines().map(str::trim) {
			if let Some(name) = line
				.strip_prefix('[')
				.and_then(|line| line.strip_suffix(']'))
			{
				in_profile = name.trim() == profile;
			} else if in_profile
				&& let Some((key, value)) = line.split_once('=')
			{
				let value = Some(SmolStr::new(value.trim()));
				match key.trim() {
					"aws_access_key_id" => access_key_id = value,
					"aws_secret_access_key" => secret_access_key = value,
					"aws_session_token" => session_token = value,
					_ => {}
				}
			}
		}
		let credentials = Self::new(access_key_id?, secret_access_key?);
		match session_token {
			Some(token) => credentials.with_session_token(token),
			None => credentials,
		}
		.xsome()
	}

	/// The credential a metadata endpoint answers (ECS, IMDS, and STS's
	/// json form): `AccessKeyId`, `SecretAccessKey`, `Token` and the
	/// `Expiration` instant.
	#[cfg(not(target_arch = "wasm32"))]
	pub(crate) fn from_metadata_json(json: &serde_json::Value) -> Result<Self> {
		let field = |name: &str| {
			json[name].as_str().map(SmolStr::new).ok_or_else(|| {
				bevyhow!("the credential answer has no `{name}`")
			})
		};
		let credentials =
			Self::new(field("AccessKeyId")?, field("SecretAccessKey")?);
		let credentials = match json["Token"].as_str() {
			Some(token) if !token.is_empty() => {
				credentials.with_session_token(token)
			}
			_ => credentials,
		};
		match json["Expiration"].as_str() {
			Some(expiration) => credentials.with_expiry(
				Timestamp::parse_rfc3339(expiration).ok_or_else(|| {
					bevyhow!("unreadable credential expiry `{expiration}`")
				})?,
			),
			None => credentials,
		}
		.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The key id is printed, the secret half and the token never.
	#[beet_core::test]
	fn debug_redacts() {
		format!(
			"{:?}",
			AwsCredentials::new("AKID", "hunter2").with_session_token("tok3n")
		)
		.xpect_contains("AKID")
		.xnot()
		.xpect_contains("hunter2")
		.xnot()
		.xpect_contains("tok3n");
	}

	/// The launch's profile in a shared credentials file, its session token
	/// included; another profile's keys are never read.
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	#[beet_core::test]
	fn reads_a_shared_profile() {
		let text = "[other]\naws_access_key_id = OTHER\n\
			aws_secret_access_key = other\n\n[default]\n\
			aws_access_key_id = AKID\naws_secret_access_key=secret\n\
			aws_session_token = token\n";
		let credentials =
			AwsCredentials::parse_profile(text, "default").unwrap();
		credentials.access_key_id().xpect_eq("AKID");
		credentials.secret_access_key().xpect_eq("secret");
		credentials.session_token().xpect_eq(Some("token"));
		AwsCredentials::parse_profile(text, "missing").xpect_none();
	}

	/// A metadata endpoint's answer, its expiry read as an instant.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	fn reads_metadata_json() {
		let credentials =
			AwsCredentials::from_metadata_json(&serde_json::json!({
				"AccessKeyId": "ASIA",
				"SecretAccessKey": "secret",
				"Token": "token",
				"Expiration": "2026-11-22T12:00:00Z"
			}))
			.unwrap();
		credentials.session_token().xpect_eq(Some("token"));
		credentials
			.expiry()
			.xpect_eq(Timestamp::parse_iso8601("2026-11-22T12:00:00Z"));
	}
}
