use crate::prelude::*;
use beet_core::prelude::*;

/// STS: who a credential belongs to, and the roles it may assume.
#[derive(Debug, Clone)]
pub struct AwsSts {
	client: AwsClient,
}

/// Who a credential belongs to, `sts:GetCallerIdentity`'s answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallerIdentity {
	/// The account id.
	pub account: SmolStr,
	/// The principal's arn, ie `arn:aws:iam::123456789012:user/beet-agent`.
	pub arn: SmolStr,
	/// The principal's unique id.
	pub user_id: SmolStr,
}

/// One `sts:AssumeRole` request.
#[derive(Clone)]
pub struct AssumeRole {
	/// The role's arn.
	pub role_arn: SmolStr,
	/// The session's name, which the role's CloudTrail entries carry.
	pub session_name: SmolStr,
	/// How long the credential lasts, clamped by STS to the role's ceiling.
	pub duration_secs: i64,
	/// The mfa device's serial (its arn) and the code it shows, where the
	/// role's trust policy requires one.
	pub mfa: Option<(SmolStr, SmolStr)>,
}

impl core::fmt::Debug for AssumeRole {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("AssumeRole")
			.field("role_arn", &self.role_arn)
			.field("session_name", &self.session_name)
			.field("duration_secs", &self.duration_secs)
			.field("mfa", &self.mfa.as_ref().map(|(serial, _)| serial))
			.finish()
	}
}

impl AwsSts {
	/// The region STS is asked in when the launch names none: it answers
	/// for the whole partition from any region, and naming one here spares a
	/// machine off EC2 the instance metadata probe for one.
	pub const DEFAULT_REGION: &'static str = "us-east-1";

	/// STS as `credentials`, in the launch's region else
	/// [`DEFAULT_REGION`](Self::DEFAULT_REGION).
	pub fn new(credentials: impl Into<AwsCredentialSource>) -> Self {
		Self::from_client(
			AwsClient::new(AwsService::STS, Self::region())
				.with_credentials(credentials),
		)
	}

	/// STS as the launch's own credential.
	pub fn launch() -> Self {
		Self::from_client(AwsClient::new(AwsService::STS, Self::region()))
	}

	/// STS through `client`, ie one with a test transport.
	pub fn from_client(client: AwsClient) -> Self { Self { client } }

	fn region() -> SmolStr {
		AwsClient::launch_region()
			.unwrap_or_else(|| Self::DEFAULT_REGION.into())
	}

	/// `sts:GetCallerIdentity`.
	pub async fn caller_identity(&self) -> Result<CallerIdentity> {
		let result = self
			.client
			.call_query("GetCallerIdentity", Vec::<(&str, &str)>::new())
			.await?;
		let field = |name: &str| {
			result.text_at(name).map(SmolStr::new).ok_or_else(|| {
				bevyhow!("sts:GetCallerIdentity answered without its `{name}`")
			})
		};
		CallerIdentity {
			account: field("Account")?,
			arn: field("Arn")?,
			user_id: field("UserId")?,
		}
		.xok()
	}

	/// The account id of the launch's own credential, read once per
	/// process.
	///
	/// A launch's account cannot change under it, so the first caller pays
	/// the round trip and every later one reads the cache; a failure is not
	/// cached, so a launch that gains credentials mid-flight resolves on the
	/// next attempt.
	pub async fn account_id() -> Result<SmolStr> {
		static ACCOUNT_ID: async_lock::OnceCell<SmolStr> =
			async_lock::OnceCell::new();
		ACCOUNT_ID
			.get_or_try_init(|| async {
				Self::launch().caller_identity().await.map(|id| id.account)
			})
			.await
			.cloned()
	}

	/// `sts:AssumeRole`, answering the role's temporary credential with its
	/// expiry. A code rides only inside this one signed request body.
	pub async fn assume_role(
		&self,
		request: &AssumeRole,
	) -> Result<AwsCredentials> {
		let mut params = vec![
			("RoleArn", request.role_arn.to_string()),
			("RoleSessionName", request.session_name.to_string()),
			("DurationSeconds", request.duration_secs.to_string()),
		];
		let code = match &request.mfa {
			Some((serial, code)) => {
				params.push(("SerialNumber", serial.to_string()));
				params.push(("TokenCode", code.to_string()));
				code.to_string()
			}
			None => String::new(),
		};
		let result = self
			.client
			.call_query_secret("AssumeRole", params, &code)
			.await?;
		Self::credentials_of(&result)
	}

	/// The credential an `AssumeRole` result carries.
	fn credentials_of(result: &BsxElement) -> Result<AwsCredentials> {
		let field =
			|name: &str| {
				result.text_at(&format!("Credentials/{name}")).ok_or_else(
					|| bevyhow!("sts:AssumeRole answered without its `{name}`"),
				)
			};
		let expiration = field("Expiration")?;
		AwsCredentials::new(field("AccessKeyId")?, field("SecretAccessKey")?)
			.with_session_token(field("SessionToken")?)
			.with_expiry(Timestamp::parse_rfc3339(expiration).ok_or_else(
				|| bevyhow!("unreadable credential expiry `{expiration}`"),
			)?)
			.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use std::sync::Arc;
	use std::sync::Mutex;

	/// The caller's identity read from its xml answer.
	#[beet_core::test]
	async fn reads_the_caller() {
		let client = AwsClient::new(AwsService::STS, "us-east-1")
			.with_credentials(AwsCredentials::new("AKID", "secret"))
			.with_transport(|_| {
				Box::pin(async {
					Response::ok()
						.with_body(
							"<GetCallerIdentityResponse xmlns=\"https://sts.amazonaws.com/doc/2011-06-15/\">\
							<GetCallerIdentityResult>\
							<Arn>arn:aws:iam::123456789012:user/beet-agent</Arn>\
							<UserId>AIDAEXAMPLE</UserId>\
							<Account>123456789012</Account>\
							</GetCallerIdentityResult>\
							<ResponseMetadata><RequestId>01234567</RequestId></ResponseMetadata>\
							</GetCallerIdentityResponse>",
						)
						.xok()
				})
			});
		AwsSts::from_client(client)
			.caller_identity()
			.await
			.unwrap()
			.xpect_eq(CallerIdentity {
				account: "123456789012".into(),
				arn: "arn:aws:iam::123456789012:user/beet-agent".into(),
				user_id: "AIDAEXAMPLE".into(),
			});
	}

	/// The code travels inside the signed body and nowhere else: not in an
	/// error, not in the request's debug form.
	#[beet_core::test]
	async fn assumes_with_a_code() {
		let sent = Arc::new(Mutex::new(Vec::<String>::new()));
		let seen = sent.clone();
		let client = AwsClient::new(AwsService::STS, "us-east-1")
			.with_credentials(AwsCredentials::new("AKID", "secret"))
			.with_transport(move |request| {
				let seen = seen.clone();
				Box::pin(async move {
					let body = request.body.into_bytes().await?;
					seen.lock()
						.unwrap()
						.push(String::from_utf8(body.to_vec())?);
					Response::ok()
						.with_body(
							"<AssumeRoleResponse><AssumeRoleResult><Credentials>\
							<AccessKeyId>ASIAEXAMPLE</AccessKeyId>\
							<SecretAccessKey>secrethalf</SecretAccessKey>\
							<SessionToken>sessiontoken</SessionToken>\
							<Expiration>2026-09-30T12:00:00Z</Expiration>\
							</Credentials></AssumeRoleResult></AssumeRoleResponse>",
						)
						.xok()
				})
			});
		let request = AssumeRole {
			role_arn: "arn:aws:iam::1234:role/beet-admin".into(),
			session_name: "beet-admin".into(),
			duration_secs: 1800,
			mfa: Some((
				"arn:aws:iam::1234:mfa/beet-agent".into(),
				"123456".into(),
			)),
		};
		format!("{request:?}").xnot().xpect_contains("123456");
		let credentials = AwsSts::from_client(client)
			.assume_role(&request)
			.await
			.unwrap();
		credentials.access_key_id().xpect_eq("ASIAEXAMPLE");
		credentials.session_token().xpect_eq(Some("sessiontoken"));
		credentials
			.expiry()
			.xpect_eq(Timestamp::parse_rfc3339("2026-09-30T12:00:00Z"));
		sent.lock().unwrap()[0].clone().xpect_eq(
			"Action=AssumeRole&Version=2011-06-15\
				&RoleArn=arn%3Aaws%3Aiam%3A%3A1234%3Arole%2Fbeet-admin\
				&RoleSessionName=beet-admin&DurationSeconds=1800\
				&SerialNumber=arn%3Aaws%3Aiam%3A%3A1234%3Amfa%2Fbeet-agent\
				&TokenCode=123456"
				.to_string(),
		);
	}

	/// A refusal that echoes the code has it redacted.
	#[beet_core::test]
	async fn redacts_the_code() {
		let client = AwsClient::new(AwsService::STS, "us-east-1")
			.with_credentials(AwsCredentials::new("AKID", "secret"))
			.with_transport(|_| {
				Box::pin(async {
					Response::from_status(StatusCode::FORBIDDEN)
						.with_body(
							"<ErrorResponse><Error><Type>Sender</Type>\
							<Code>AccessDenied</Code>\
							<Message>MultiFactorAuthentication failed with invalid MFA one time pass code 654321</Message>\
							</Error></ErrorResponse>",
						)
						.xok()
				})
			});
		let err = AwsSts::from_client(client)
			.assume_role(&AssumeRole {
				role_arn: "arn:aws:iam::1234:role/beet-admin".into(),
				session_name: "beet-admin".into(),
				duration_secs: 1800,
				mfa: Some((
					"arn:aws:iam::1234:mfa/beet-agent".into(),
					"654321".into(),
				)),
			})
			.await
			.unwrap_err();
		AwsError::code(&err).xpect_eq(Some("AccessDenied"));
		err.to_string()
			.xpect_contains("MultiFactorAuthentication failed")
			.xnot()
			.xpect_contains("654321");
	}
}
