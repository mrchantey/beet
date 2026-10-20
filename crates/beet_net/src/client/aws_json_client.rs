//! An AWS JSON api as signed requests through beet's own http client.

use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::Value;
use std::sync::Arc;

/// One AWS service speaking the JSON 1.1 protocol (parameter store, and
/// any other `X-Amz-Target` api) in one region, called as Signature Version
/// 4 requests through [`Request::send`]: no SDK client, no runtime and no
/// `aws` cli, so it builds on every target, wasm included.
///
/// Each call signs as [`AwsCredentials::discover`] (the launch's own, ie a
/// box's pair or a deployer's) unless the client holds explicit ones, and
/// is retried on a transport failure (an answer from AWS, error or not, is
/// never retried). Patience is the transport's: the native one allows a
/// 30s connect with no deadline on the lookup, which outlasts a resolver
/// stalled on an unreachable nameserver, where the SDK's 3.1s default
/// connect failed (`aws_ext::STALL_TIMEOUT` is that guard for SDK clients).
///
/// Nothing here logs a request body or a response, and
/// [`call_secret`](Self::call_secret) redacts the secret a write carries
/// from any error it answers.
#[derive(Clone)]
pub struct AwsJsonClient {
	/// The signing name and endpoint prefix, ie `ssm`.
	service: SmolStr,
	/// The `X-Amz-Target` prefix, ie `AmazonSSM`.
	target_prefix: SmolStr,
	region: SmolStr,
	endpoint: SmolStr,
	credentials: Option<AwsCredentials>,
	transport: Option<AwsTransport>,
}

/// How a signed request reaches AWS: [`Request::send`] unless a test
/// installs a handler.
type AwsTransport = Arc<
	dyn 'static
		+ Send
		+ Sync
		+ Fn(Request) -> SendBoxedFuture<Result<Response>>,
>;

impl std::fmt::Debug for AwsJsonClient {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("AwsJsonClient")
			.field("service", &self.service)
			.field("region", &self.region)
			.field("endpoint", &self.endpoint)
			.finish_non_exhaustive()
	}
}

impl AwsJsonClient {
	/// The content type of every JSON 1.1 request.
	pub const CONTENT_TYPE: &'static str = "application/x-amz-json-1.1";

	/// `service` in `region` at its public endpoint, ie
	/// `https://ssm.ap-southeast-2.amazonaws.com/`, targeting
	/// `<target_prefix>.<action>`.
	pub fn new(
		service: impl Into<SmolStr>,
		target_prefix: impl Into<SmolStr>,
		region: impl Into<SmolStr>,
	) -> Self {
		let (service, region) = (service.into(), region.into());
		Self {
			endpoint: format!("https://{service}.{region}.amazonaws.com/")
				.into(),
			service,
			target_prefix: target_prefix.into(),
			region,
			credentials: None,
			transport: None,
		}
	}

	/// Sign as `credentials` rather than the launch's own.
	pub fn with_credentials(mut self, credentials: AwsCredentials) -> Self {
		self.credentials = Some(credentials);
		self
	}

	/// Send every request through `handler` rather than the network: a
	/// test's stand-in for AWS.
	pub fn with_transport(
		mut self,
		handler: impl 'static
		+ Send
		+ Sync
		+ Fn(Request) -> SendBoxedFuture<Result<Response>>,
	) -> Self {
		self.transport = Some(Arc::new(handler));
		self
	}

	/// The region every request goes to.
	pub fn region(&self) -> &str { &self.region }

	/// Call `action` with `body`, answering the response's json; an answer
	/// AWS marks as an error is an [`AwsError`].
	pub async fn call(&self, action: &str, body: &Value) -> Result<Value> {
		self.send(action, body, None).await
	}

	/// [`call`](Self::call) for a write carrying `secret`, which is
	/// redacted from any error it answers.
	pub async fn call_secret(
		&self,
		action: &str,
		body: &Value,
		secret: &str,
	) -> Result<Value> {
		self.send(action, body, Some(secret)).await
	}

	async fn send(
		&self,
		action: &str,
		body: &Value,
		secret: Option<&str>,
	) -> Result<Value> {
		let body = serde_json::to_vec(body)?;
		let credentials = match &self.credentials {
			Some(credentials) => credentials.clone(),
			None => AwsCredentials::discover()?,
		};
		// the waits up front, so no rng is held across an await and the
		// future stays `Send`
		let waits = Backoff::default()
			.with_max_attempts(3)
			.iter()
			.map(|frame| frame.next_attempt)
			.collect::<Vec<_>>();
		let mut sent = Err(bevyhow!("no attempt made"));
		for wait in waits {
			let request = credentials
				.sign(
					Request::post(self.endpoint.as_str())
						.with_header_raw("content-type", Self::CONTENT_TYPE)
						.with_header_raw(
							"x-amz-target",
							&format!("{}.{action}", self.target_prefix),
						)
						.with_body(body.clone()),
					&self.region,
					&self.service,
					Timestamp::now(),
				)
				.await?;
			sent = match &self.transport {
				Some(transport) => transport(request).await,
				None => request.send().await,
			};
			match (&sent, wait) {
				(Err(err), Some(wait)) => {
					warn!(
						"{} {action} did not reach AWS, retrying in {wait:?}: {err}",
						self.service
					);
					cfg_if! {
						if #[cfg(target_arch = "wasm32")] {
							// a js timer, polled only on the one thread that
							// made it, so the call stays `Send` for a provider
							send_wrapper::SendWrapper::new(time_ext::sleep(wait))
								.await;
						} else {
							time_ext::sleep(wait).await;
						}
					}
				}
				_ => break,
			}
		}
		let response = sent?;
		let status = response.status();
		let text = response.text().await?;
		if status.is_success() {
			return match text.trim().is_empty() {
				true => Value::Null.xok(),
				false => serde_json::from_str::<Value>(&text)?.xok(),
			};
		}
		let redact = |text: &str| match secret {
			Some(secret) if !secret.is_empty() => text.replace(secret, "***"),
			_ => text.to_string(),
		};
		let error = serde_json::from_str::<Value>(&text).unwrap_or_default();
		Err(AwsError {
			service: self.service.clone(),
			action: action.into(),
			status: status.as_u16(),
			code: AwsError::code_of(&error).into(),
			message: redact(
				error["message"]
					.as_str()
					.or_else(|| error["Message"].as_str())
					.unwrap_or(&text),
			),
		}
		.into())
	}
}

/// [`AwsJsonClient::call`] is `Send` on every target, the guarantee a
/// provider whose futures must be `Send` (a secret store) relies on.
#[allow(dead_code)]
fn call_is_send() {
	fn assert_send(_: impl Send) {}
	let client = AwsJsonClient::new("ssm", "AmazonSSM", "us-west-2");
	assert_send(client.call("GetParameter", &Value::Null));
}

/// An answer AWS marks as an error, ie `ParameterNotFound`: matched on by
/// [`code`](Self::code), the one part of an AWS error that is stable.
#[derive(Debug, thiserror::Error)]
#[error("{service} {action} answered {status} {code}: {message}")]
pub struct AwsError {
	service: SmolStr,
	action: SmolStr,
	status: u16,
	code: SmolStr,
	message: String,
}

impl AwsError {
	/// The error code `err` carries when it is an [`AwsError`], ie
	/// `ParameterNotFound`.
	pub fn code(err: &BevyError) -> Option<&str> {
		err.downcast_ref::<Self>().map(|err| err.code.as_str())
	}

	/// The code of an error body: its `__type`, past any namespace
	/// (`com.amazonaws.ssm#ParameterNotFound`).
	fn code_of(error: &Value) -> &str {
		error["__type"]
			.as_str()
			.map(|code| code.rsplit('#').next().unwrap_or(code))
			.unwrap_or("Unknown")
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use serde_json::json;

	/// A client whose AWS is `handler`, signing as a fixed credential.
	fn client(
		handler: impl 'static
		+ Send
		+ Sync
		+ Fn(Request) -> SendBoxedFuture<Result<Response>>,
	) -> AwsJsonClient {
		AwsJsonClient::new("ssm", "AmazonSSM", "ap-southeast-2")
			.with_credentials(AwsCredentials::new("AKID", "secret"))
			.with_transport(handler)
	}

	/// A call is a signed POST of the json body to the regional endpoint,
	/// targeting the action, and answers the response json.
	#[beet_core::test]
	async fn calls_the_target() {
		client(|request| {
			Box::pin(async move {
				request
					.url()
					.to_string()
					.xpect_eq("https://ssm.ap-southeast-2.amazonaws.com/");
				request
					.headers
					.first_raw("x-amz-target")
					.unwrap()
					.xpect_eq("AmazonSSM.DescribeParameters");
				request
					.headers
					.first_raw("authorization")
					.unwrap()
					.xpect_contains("/ap-southeast-2/ssm/aws4_request");
				let body = request.body.into_bytes().await?;
				serde_json::from_slice::<serde_json::Value>(&body)?
					.xpect_eq(json!({"MaxResults": 50}));
				Response::ok().with_body(r#"{"Parameters":[]}"#).xok()
			})
		})
		.call("DescribeParameters", &json!({"MaxResults": 50}))
		.await
		.unwrap()
		.xpect_eq(json!({"Parameters": []}));
	}

	/// An error answer is an `AwsError` by its code, with the secret a write
	/// carries redacted from its message.
	#[beet_core::test]
	async fn answers_errors_by_code() {
		let err = client(|_| {
			Box::pin(async {
				Response::from_status(StatusCode::BAD_REQUEST)
					.with_body(
						r#"{"__type":"com.amazonaws.ssm#ParameterAlreadyExists","message":"value hunter2 exists"}"#,
					)
					.xok()
			})
		})
		.call_secret("PutParameter", &json!({"Value": "hunter2"}), "hunter2")
		.await
		.unwrap_err();
		AwsError::code(&err).xpect_eq(Some("ParameterAlreadyExists"));
		err.to_string()
			.xpect_contains("PutParameter")
			.xnot()
			.xpect_contains("hunter2");
	}
}
