//! The AWS identity a request is signed as, and the signing.

use crate::prelude::*;
use aws_sigv4::http_request::SignableBody;
use aws_sigv4::http_request::SignableRequest;
use aws_sigv4::http_request::SigningSettings;
use aws_sigv4::sign::v4::SigningParams;
use aws_smithy_runtime_api::client::identity::Identity;
use beet_core::prelude::*;
use std::time::SystemTime;

/// The AWS identity a request is signed as: an access key pair and, for a
/// temporary credential, its session token. [`Debug`] prints the key id
/// alone.
///
/// [`discover`](Self::discover) finds the launch's own, the chain a box and
/// a deployer share: the environment (a box's unit loads its pair through
/// an `EnvironmentFile`, a deployer's launch loads the `env_var` records of
/// its secrets document), else the shared credentials file the `aws` cli
/// and the SDK read. [`sign`](Self::sign) signs a request with Signature
/// Version 4, over beet's own [`Request`], so any target that can send one
/// can reach an AWS api.
#[derive(Clone)]
pub struct AwsCredentials {
	access_key_id: SmolStr,
	secret_access_key: SmolStr,
	session_token: Option<SmolStr>,
}

impl std::fmt::Debug for AwsCredentials {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("AwsCredentials")
			.field("access_key_id", &self.access_key_id)
			.finish_non_exhaustive()
	}
}

impl AwsCredentials {
	/// The variable holding the key id, the first link of the chain.
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
		}
	}

	/// The same pair as a temporary credential carrying `token`.
	pub fn with_session_token(mut self, token: impl Into<SmolStr>) -> Self {
		self.session_token = Some(token.into());
		self
	}

	/// The key id, the one half safe to print.
	pub fn access_key_id(&self) -> &str { &self.access_key_id }

	/// The launch's own credential: the environment's pair, else the
	/// shared credentials file's profile (`AWS_PROFILE`, else `default`, at
	/// `AWS_SHARED_CREDENTIALS_FILE`, else `~/.aws/credentials`). An error
	/// naming both when neither holds one.
	pub fn discover() -> Result<Self> {
		if let Some(credentials) = Self::from_env() {
			return credentials.xok();
		}
		#[cfg(feature = "fs")]
		if let Some(credentials) = Self::from_shared_file()? {
			return credentials.xok();
		}
		bevybail!(
			"no AWS credentials: neither `{}`/`{}` in the environment nor a \
			profile in the shared credentials file",
			Self::ACCESS_KEY_ID,
			Self::SECRET_ACCESS_KEY
		)
	}

	/// The environment's pair, `None` unless both halves are set.
	fn from_env() -> Option<Self> {
		let access_key_id = env_ext::var(Self::ACCESS_KEY_ID).ok()?;
		let secret_access_key = env_ext::var(Self::SECRET_ACCESS_KEY).ok()?;
		let credentials = Self::new(access_key_id, secret_access_key);
		match env_ext::var(Self::SESSION_TOKEN) {
			Ok(token) => credentials.with_session_token(token),
			Err(_) => credentials,
		}
		.xmap(Some)
	}

	/// The launch's profile in the shared credentials file, `None` when
	/// there is no file or it lacks the profile.
	#[cfg(feature = "fs")]
	fn from_shared_file() -> Result<Option<Self>> {
		let path = match env_ext::var("AWS_SHARED_CREDENTIALS_FILE") {
			Ok(path) => std::path::PathBuf::from(path.as_str()),
			Err(_) => match fs_ext::home_dir() {
				Ok(home) => home.join(".aws").join("credentials"),
				Err(_) => return Ok(None),
			},
		};
		let Ok(text) = fs_ext::read_to_string(&path) else {
			return Ok(None);
		};
		let profile =
			env_ext::var("AWS_PROFILE").unwrap_or_else(|_| "default".into());
		Self::parse_profile(&text, &profile).xok()
	}

	/// `profile`'s pair in a shared credentials file's text.
	#[cfg(feature = "fs")]
	fn parse_profile(text: &str, profile: &str) -> Option<Self> {
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
		.xmap(Some)
	}

	/// `request` signed with Signature Version 4 for `service` in `region`
	/// at `at`: its `x-amz-date`, the session token where this credential
	/// carries one, and the `authorization` over every header it already
	/// carries, its body and the host its url names. Signed in the form the
	/// transport sends it (the same conversion to an `http` request), so the
	/// signature covers exactly what goes on the wire.
	pub async fn sign(
		&self,
		request: Request,
		region: &str,
		service: &str,
		at: Timestamp,
	) -> Result<Request> {
		let (mut parts, body) = request.into_parts();
		let body = body.into_bytes().await?;
		let wire: http::request::Parts = parts.clone().try_into()?;
		let headers = wire
			.headers
			.iter()
			.map(|(name, value)| Ok((name.as_str(), value.to_str()?)))
			.collect::<Result<Vec<_>>>()?;
		let identity: Identity = aws_credential_types::Credentials::new(
			self.access_key_id.as_str(),
			self.secret_access_key.as_str(),
			self.session_token.as_ref().map(SmolStr::to_string),
			None,
			"beet",
		)
		.into();
		let params = SigningParams::builder()
			.identity(&identity)
			.region(region)
			.name(service)
			.time(
				SystemTime::UNIX_EPOCH
					+ Duration::from_millis(at.millis().max(0) as u64),
			)
			.settings(SigningSettings::default())
			.build()?
			.into();
		let signable = SignableRequest::new(
			wire.method.as_str(),
			wire.uri.to_string(),
			headers.into_iter(),
			SignableBody::Bytes(&body),
		)?;
		let (instructions, _signature) =
			aws_sigv4::http_request::sign(signable, &params)?.into_parts();
		for (name, value) in instructions.headers() {
			parts.headers.set_raw(name, value);
		}
		Request::from_parts(parts, body.into()).xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The credential of AWS's published Signature Version 4 test suite.
	fn suite_credentials() -> AwsCredentials {
		AwsCredentials::new(
			"AKIDEXAMPLE",
			"wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
		)
	}

	/// The suite's clock, `20150830T123600Z`.
	fn suite_time() -> Timestamp {
		Timestamp::parse_iso8601("2015-08-30T12:36:00Z").unwrap()
	}

	/// `get-vanilla` from AWS's Signature Version 4 test suite: the
	/// published signature for a bare `GET /`.
	#[beet_core::test]
	async fn signs_the_published_vector() {
		suite_credentials()
			.sign(
				Request::get("https://example.amazonaws.com/"),
				"us-east-1",
				"service",
				suite_time(),
			)
			.await
			.unwrap()
			.headers
			.first_raw("authorization")
			.unwrap()
			.xpect_eq(
				"AWS4-HMAC-SHA256 \
				Credential=AKIDEXAMPLE/20150830/us-east-1/service/aws4_request, \
				SignedHeaders=host;x-amz-date, \
				Signature=5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31",
			);
	}

	/// A parameter store call as the provider signs one: the content type,
	/// the target and the session token are signed beside the host and the
	/// date, and the body is hashed in. The signature is a fixture computed
	/// independently of this crate (Python's `hmac` over the canonical
	/// request), so a change in what is signed is caught here.
	#[beet_core::test]
	async fn signs_a_json_call() {
		let request =
			Request::post("https://ssm.ap-southeast-2.amazonaws.com/")
				.with_header_raw("content-type", "application/x-amz-json-1.1")
				.with_header_raw("x-amz-target", "AmazonSSM.GetParameter")
				.with_body(r#"{"Name":"/app/prod/x","WithDecryption":true}"#);
		let signed = suite_credentials()
			.with_session_token("TOKEN")
			.sign(request, "ap-southeast-2", "ssm", suite_time())
			.await
			.unwrap();
		signed
			.headers
			.first_raw("x-amz-date")
			.unwrap()
			.xpect_eq("20150830T123600Z");
		signed
			.headers
			.first_raw("x-amz-security-token")
			.unwrap()
			.xpect_eq("TOKEN");
		signed
			.headers
			.first_raw("authorization")
			.unwrap()
			.xpect_eq(
				"AWS4-HMAC-SHA256 \
				Credential=AKIDEXAMPLE/20150830/ap-southeast-2/ssm/aws4_request, \
				SignedHeaders=content-type;host;x-amz-date;x-amz-security-token;x-amz-target, \
				Signature=08d0f5fc85d81cf05c7636ff5ccd41b412f1c0587f634ca9bbb9ecbb7e47bbe8",
			);
		// the body is untouched by the signing
		signed
			.body
			.into_bytes()
			.await
			.unwrap()
			.as_ref()
			.xpect_eq(br#"{"Name":"/app/prod/x","WithDecryption":true}"#);
	}

	/// The launch's profile in a shared credentials file, its session token
	/// included; another profile's keys are never read.
	#[cfg(feature = "fs")]
	#[beet_core::test]
	fn reads_a_shared_profile() {
		let text = "[other]\naws_access_key_id = OTHER\n\
			aws_secret_access_key = other\n\n[default]\n\
			aws_access_key_id = AKID\naws_secret_access_key=secret\n\
			aws_session_token = token\n";
		let credentials =
			AwsCredentials::parse_profile(text, "default").unwrap();
		format!("{credentials:?}")
			.xpect_contains("AKID")
			.xnot()
			.xpect_contains("secret");
		credentials.access_key_id().xpect_eq("AKID");
		credentials
			.session_token
			.unwrap()
			.as_str()
			.xpect_eq("token");
		AwsCredentials::parse_profile(text, "missing").xpect_none();
	}
}
