use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::Value;

/// An answer AWS marks as an error, ie `ParameterNotFound`: matched on by
/// [`code`](Self::code), the one part of an AWS error that is stable.
///
/// Read from whichever shape the protocol answers in: json (`__type` or
/// `code`, `message` or `Message`), xml (`<Error><Code>`, EC2's
/// `<Response><Errors><Error>`), the `x-amzn-errortype` and
/// `x-amzn-query-error` headers, or for an answer with no body (a `HEAD`) the
/// status alone.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{service} {action} answered {status} {code}: {message}")]
pub struct AwsError {
	service: SmolStr,
	action: SmolStr,
	status: u16,
	code: SmolStr,
	message: String,
}

impl AwsError {
	/// The code a status implies when the answer carried none.
	fn code_of_status(status: u16) -> &'static str {
		match status {
			301 => "PermanentRedirect",
			304 => "NotModified",
			400 => "BadRequest",
			403 => "Forbidden",
			404 => "NotFound",
			412 => "PreconditionFailed",
			_ => "Unknown",
		}
	}

	/// The error `response` carries, its body read as `protocol` answers.
	pub(crate) fn read(
		service: &AwsService,
		action: &str,
		status: StatusCode,
		headers: &HeaderMap,
		body: &str,
	) -> Self {
		let (code, message) = match service.protocol {
			AwsProtocol::RestXml
			| AwsProtocol::AwsQuery { .. }
			| AwsProtocol::Ec2Query { .. } => Self::from_xml(body),
			_ => Self::from_json(body),
		};
		// a query-compatible json service names its query code in a header,
		// and a rest service may name the code only there
		let header_code = headers
			.first_raw("x-amzn-query-error")
			.and_then(|value| value.split(';').next())
			.or_else(|| headers.first_raw("x-amzn-errortype"))
			.map(Self::strip_namespace);
		let code = code
			.or(header_code.map(SmolStr::new))
			.unwrap_or_else(|| Self::code_of_status(status.as_u16()).into());
		Self {
			service: service.endpoint_prefix.into(),
			action: action.into(),
			status: status.as_u16(),
			code,
			message: message.unwrap_or_else(|| match body.trim().is_empty() {
				true => status.to_string(),
				false => body.trim().chars().take(512).collect(),
			}),
		}
	}

	/// A json error's code and message.
	fn from_json(body: &str) -> (Option<SmolStr>, Option<String>) {
		let Ok(json) = serde_json::from_str::<Value>(body) else {
			return (None, None);
		};
		let code = ["__type", "code", "Code"]
			.iter()
			.find_map(|key| json[*key].as_str())
			.map(|code| SmolStr::new(Self::strip_namespace(code)));
		let message = ["message", "Message", "errorMessage"]
			.iter()
			.find_map(|key| json[*key].as_str())
			.map(ToString::to_string);
		(code, message)
	}

	/// An xml error's code and message, wherever its `Error` element sits.
	fn from_xml(body: &str) -> (Option<SmolStr>, Option<String>) {
		let Ok(xml) = XmlElement::parse(body) else {
			return (None, None);
		};
		let error = match xml.local_name() == "Error" {
			true => Some(&xml),
			false => xml.find("Error"),
		};
		match error {
			Some(error) => (
				error.text_at("Code").map(SmolStr::new),
				error.text_at("Message").map(ToString::to_string),
			),
			None => (None, None),
		}
	}

	/// A code past any namespace or suffix:
	/// `com.amazonaws.ssm#ParameterNotFound:http://..` is `ParameterNotFound`.
	fn strip_namespace(code: &str) -> &str {
		let code = code.rsplit('#').next().unwrap_or(code);
		code.split(':').next().unwrap_or(code)
	}

	/// The same error with `secret` redacted from its message.
	pub(crate) fn redact(mut self, secret: Option<&str>) -> Self {
		if let Some(secret) = secret.filter(|secret| !secret.is_empty()) {
			self.message = self.message.replace(secret, "***");
		}
		self
	}

	/// The error `err` is, when it is one.
	pub fn of(err: &BevyError) -> Option<&Self> { err.downcast_ref::<Self>() }

	/// The error code `err` carries when it is an [`AwsError`], ie
	/// `ParameterNotFound`.
	pub fn code(err: &BevyError) -> Option<&str> {
		Self::of(err).map(|err| err.code.as_str())
	}

	/// Whether `err` is an [`AwsError`] with one of `codes`.
	pub fn is(err: &BevyError, codes: &[&str]) -> bool {
		Self::code(err).is_some_and(|code| codes.contains(&code))
	}

	/// The error code.
	pub fn error_code(&self) -> &str { &self.code }

	/// The http status.
	pub fn status(&self) -> u16 { self.status }

	/// The server's explanation, redacted of the secret a write carried.
	pub fn message(&self) -> &str { &self.message }

	/// Whether a retry may succeed: a throttle or a server fault.
	pub fn is_retryable(&self) -> bool {
		self.status >= 500 || Self::THROTTLING.contains(&self.code.as_str())
	}

	/// The codes AWS answers a throttled request with.
	pub const THROTTLING: &'static [&'static str] = &[
		"Throttling",
		"ThrottlingException",
		"ThrottledException",
		"RequestThrottled",
		"RequestLimitExceeded",
		"TooManyRequestsException",
		"ProvisionedThroughputExceededException",
		"SlowDown",
		"RequestThrottledException",
		"TransactionInProgressException",
		"ConcurrentModificationException",
	];
}

#[cfg(test)]
mod test {
	use super::*;

	fn read(
		service: AwsService,
		status: u16,
		headers: &[(&str, &str)],
		body: &str,
	) -> AwsError {
		let mut map = HeaderMap::new();
		for (name, value) in headers {
			map.set_raw(*name, *value);
		}
		AwsError::read(&service, "Action", StatusCode::new(status), &map, body)
	}

	/// Every shape an error arrives in names its code.
	#[beet_core::test]
	fn reads_every_shape() {
		read(
			AwsService::SSM,
			400,
			&[],
			r#"{"__type":"com.amazonaws.ssm#ParameterNotFound","message":"gone"}"#,
		)
		.xmap(|err| (err.code.clone(), err.message.clone()))
		.xpect_eq(("ParameterNotFound".into(), "gone".into()));
		read(
			AwsService::S3,
			404,
			&[],
			"<Error><Code>NoSuchKey</Code><Message>The key</Message></Error>",
		)
		.code
		.as_str()
		.xpect_eq("NoSuchKey");
		read(
			AwsService::EC2,
			400,
			&[],
			"<Response><Errors><Error><Code>DryRunOperation</Code>\
			<Message>would have</Message></Error></Errors></Response>",
		)
		.code
		.as_str()
		.xpect_eq("DryRunOperation");
		read(
			AwsService::STS,
			403,
			&[],
			"<ErrorResponse><Error><Type>Sender</Type><Code>AccessDenied</Code>\
			<Message>no</Message></Error></ErrorResponse>",
		)
		.code
		.as_str()
		.xpect_eq("AccessDenied");
		read(AwsService::S3, 404, &[], "")
			.code
			.as_str()
			.xpect_eq("NotFound");
		read(
			AwsService::LAMBDA,
			404,
			&[("x-amzn-errortype", "ResourceNotFoundException:http://x")],
			r#"{"Message":"no fn"}"#,
		)
		.code
		.as_str()
		.xpect_eq("ResourceNotFoundException");
		read(
			AwsService::MONITORING,
			400,
			&[("x-amzn-query-error", "InvalidParameterValue;Sender")],
			"{}",
		)
		.code
		.as_str()
		.xpect_eq("InvalidParameterValue");
	}

	/// A write's secret never survives into the error.
	#[beet_core::test]
	fn redacts_the_secret() {
		read(
			AwsService::SSM,
			400,
			&[],
			r#"{"__type":"X","message":"value hunter2 exists"}"#,
		)
		.redact(Some("hunter2"))
		.to_string()
		.xpect_contains("***")
		.xnot()
		.xpect_contains("hunter2");
	}
}
