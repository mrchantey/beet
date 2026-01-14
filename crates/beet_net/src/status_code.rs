//! Transport-independent status codes for request/response exchanges.
//!
//! The [`StatusCode`] enum provides a general-purpose status representation
//! that works across HTTP, CLI, RPC, and other transports.

use beet_core::prelude::*;

/// A transport-independent status code for request/response exchanges.
///
/// This enum represents outcomes that can occur across different transports
/// (HTTP, CLI, RPC, etc.) without forcing non-HTTP contexts to pretend they're HTTP.
///
/// # Examples
///
/// ```rust
/// # use beet_net::prelude::*;
/// // Use semantic variants
/// let ok = StatusCode::Ok;
/// assert!(ok.is_ok());
///
/// // Map from HTTP status codes
/// #[cfg(feature = "http")]
/// let http_ok = StatusCode::Http(http::StatusCode::OK);
/// #[cfg(feature = "http")]
/// assert!(http_ok.is_ok());
///
/// // Convert to process exit codes
/// let exit_code: u8 = StatusCode::Ok.into();
/// assert_eq!(exit_code, 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StatusCode {
	/// Operation completed successfully (exit code: 0).
	#[default]
	Ok,

	/// Resource created successfully (exit code: 0).
	///
	/// Similar to HTTP 201 Created.
	Created,

	/// Request was malformed or invalid (exit code: 64).
	///
	/// Similar to HTTP 400 Bad Request.
	MalformedRequest,

	/// Authentication required or failed (exit code: 77).
	///
	/// Similar to HTTP 401 Unauthorized.
	Unauthorized,

	/// Access forbidden, credentials insufficient (exit code: 77).
	///
	/// Similar to HTTP 403 Forbidden.
	Forbidden,

	/// Requested resource not found (exit code: 1).
	///
	/// Similar to HTTP 404 Not Found.
	NotFound,

	/// Request method not allowed (exit code: 1).
	///
	/// Similar to HTTP 405 Method Not Allowed.
	MethodNotAllowed,

	/// Request timeout occurred (exit code: 75).
	///
	/// Similar to HTTP 408 Request Timeout.
	RequestTimeout,

	/// Resource conflict (exit code: 1).
	///
	/// Similar to HTTP 409 Conflict.
	Conflict,

	/// Request payload too large (exit code: 1).
	///
	/// Similar to HTTP 413 Payload Too Large.
	PayloadTooLarge,

	/// Rate limit exceeded (exit code: 75).
	///
	/// Similar to HTTP 429 Too Many Requests.
	RateLimitExceeded,

	/// Internal server or application error (exit code: 70).
	///
	/// Similar to HTTP 500 Internal Server Error.
	InternalError,

	/// Feature not implemented (exit code: 1).
	///
	/// Similar to HTTP 501 Not Implemented.
	NotImplemented,

	/// Upstream service unavailable (exit code: 69).
	///
	/// Similar to HTTP 503 Service Unavailable.
	ServiceUnavailable,

	/// Gateway or upstream timeout (exit code: 75).
	///
	/// Similar to HTTP 504 Gateway Timeout.
	GatewayTimeout,

	/// No response received from service (exit code: 69).
	///
	/// Distinct from timeout - connection succeeded but no response data.
	NoResponse,

	/// Request was cancelled (exit code: 1).
	///
	/// Operation was cancelled before completion.
	Cancelled,

	/// Resource already exists (exit code: 73).
	///
	/// Attempt to create something that already exists.
	AlreadyExists,

	/// Precondition failed (exit code: 1).
	///
	/// Similar to HTTP 412 Precondition Failed.
	PreconditionFailed,

	/// I'm a teapot (exit code: 1).
	///
	/// Similar to HTTP 418 I'm a Teapot.
	ImATeapot,

	/// Resource moved permanently (exit code: 0).
	///
	/// Similar to HTTP 301 Moved Permanently.
	MovedPermanently,

	/// Resource temporarily redirected (exit code: 0).
	///
	/// Similar to HTTP 307 Temporary Redirect.
	TemporaryRedirect,

	/// Invalid state for operation (exit code: 1).
	///
	/// Operation cannot be performed in current state.
	InvalidState,

	/// Resource exhausted (exit code: 73).
	///
	/// Out of quota, disk space, memory, etc.
	ResourceExhausted,

	/// Data loss or corruption detected (exit code: 74).
	DataLoss,

	/// Invalid argument provided (exit code: 64).
	InvalidArgument,

	/// Upstream dependency failed (exit code: 1).
	UpstreamFailure,

	/// A raw HTTP status code (may be 2XX success).
	#[cfg(feature = "http")]
	#[cfg_attr(
		feature = "serde",
		serde(serialize_with = "serialize_http_status")
	)]
	#[cfg_attr(
		feature = "serde",
		serde(deserialize_with = "deserialize_http_status")
	)]
	Http(http::StatusCode),

	/// A raw process exit code (may be 0 success).
	Process(u8),
}

impl std::fmt::Display for StatusCode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Ok => write!(f, "OK"),
			Self::Created => write!(f, "Created"),
			Self::MalformedRequest => write!(f, "Malformed Request"),
			Self::Unauthorized => write!(f, "Unauthorized"),
			Self::Forbidden => write!(f, "Forbidden"),
			Self::NotFound => write!(f, "Not Found"),
			Self::MethodNotAllowed => write!(f, "Method Not Allowed"),
			Self::RequestTimeout => write!(f, "Request Timeout"),
			Self::Conflict => write!(f, "Conflict"),
			Self::PayloadTooLarge => write!(f, "Payload Too Large"),
			Self::RateLimitExceeded => write!(f, "Rate Limit Exceeded"),
			Self::PreconditionFailed => write!(f, "Precondition Failed"),
			Self::ImATeapot => write!(f, "I'm a Teapot"),
			Self::MovedPermanently => write!(f, "Moved Permanently"),
			Self::TemporaryRedirect => write!(f, "Temporary Redirect"),
			Self::InternalError => write!(f, "Internal Error"),
			Self::NotImplemented => write!(f, "Not Implemented"),
			Self::ServiceUnavailable => write!(f, "Service Unavailable"),
			Self::GatewayTimeout => write!(f, "Gateway Timeout"),
			Self::NoResponse => write!(f, "No Response"),
			Self::Cancelled => write!(f, "Cancelled"),
			Self::AlreadyExists => write!(f, "Already Exists"),
			Self::InvalidState => write!(f, "Invalid State"),
			Self::ResourceExhausted => write!(f, "Resource Exhausted"),
			Self::DataLoss => write!(f, "Data Loss"),
			Self::InvalidArgument => write!(f, "Invalid Argument"),
			Self::UpstreamFailure => write!(f, "Upstream Failure"),
			#[cfg(feature = "http")]
			Self::Http(status) => write!(f, "{}", status),
			Self::Process(code) => write!(f, "Process Exit Code {}", code),
		}
	}
}

impl StatusCode {
	/// Returns `true` if this status represents success.
	///
	/// Checks for:
	/// - `StatusCode::Ok`
	/// - `StatusCode::Http` with 2XX range or 3XX redirects
	/// - `StatusCode::Process` with exit code 0
	/// - Redirect variants
	pub fn is_ok(&self) -> bool {
		match self {
			Self::Ok
			| Self::Created
			| Self::MovedPermanently
			| Self::TemporaryRedirect => true,
			#[cfg(feature = "http")]
			Self::Http(status) => status.is_success() || status.is_redirection(),
			Self::Process(code) => *code == 0,
			_ => false,
		}
	}

	/// Creates a StatusCode from a raw HTTP status code number.
	///
	/// Maps common HTTP codes to semantic variants, otherwise wraps in `Http`.
	#[cfg(feature = "http")]
	pub fn from_http_raw(http_status_code: u16) -> Self {
		match http_status_code {
			200 => Self::Ok,
			201 => Self::Created,
			301 => Self::MovedPermanently,
			307 => Self::TemporaryRedirect,
			400 => Self::MalformedRequest,
			401 => Self::Unauthorized,
			403 => Self::Forbidden,
			404 => Self::NotFound,
			405 => Self::MethodNotAllowed,
			408 => Self::RequestTimeout,
			409 => Self::Conflict,
			412 => Self::PreconditionFailed,
			413 => Self::PayloadTooLarge,
			418 => Self::ImATeapot,
			429 => Self::RateLimitExceeded,
			500 => Self::InternalError,
			501 => Self::NotImplemented,
			503 => Self::ServiceUnavailable,
			504 => Self::GatewayTimeout,
			_ => {
				// Fallback to Http variant for unknown codes
				http::StatusCode::from_u16(http_status_code)
					.map(Self::Http)
					.unwrap_or(Self::InternalError)
			}
		}
	}

	/// Returns `true` if this status represents an error.
	pub fn is_err(&self) -> bool { !self.is_ok() }

	/// Converts status to exit code convention result.
	///
	/// Returns `Ok(())` for success (exit code 0), or `Err(NonZeroU8)` for errors.
	pub fn to_exit_code(&self) -> Result<(), std::num::NonZeroU8> {
		let code: u8 = (*self).into();
		if code == 0 {
			Ok(())
		} else {
			Err(std::num::NonZeroU8::new(code).unwrap())
		}
	}

	/// Returns `true` if this is a client error (4XX or semantic equivalent).
	pub fn is_client_error(&self) -> bool {
		match self {
			Self::MalformedRequest
			| Self::Unauthorized
			| Self::Forbidden
			| Self::NotFound
			| Self::MethodNotAllowed
			| Self::RequestTimeout
			| Self::Conflict
			| Self::PayloadTooLarge
			| Self::RateLimitExceeded
			| Self::PreconditionFailed
			| Self::ImATeapot
			| Self::InvalidArgument => true,
			#[cfg(feature = "http")]
			Self::Http(status) => status.is_client_error(),
			_ => false,
		}
	}

	/// Returns `true` if this is a server error (5XX or semantic equivalent).
	pub fn is_server_error(&self) -> bool {
		match self {
			Self::InternalError
			| Self::NotImplemented
			| Self::ServiceUnavailable
			| Self::GatewayTimeout
			| Self::NoResponse
			| Self::UpstreamFailure => true,
			#[cfg(feature = "http")]
			Self::Http(status) => status.is_server_error(),
			_ => false,
		}
	}
}

#[cfg(feature = "http")]
impl From<http::StatusCode> for StatusCode {
	fn from(status: http::StatusCode) -> Self {
		Self::from_http_raw(status.as_u16())
	}
}

#[cfg(feature = "http")]
impl From<StatusCode> for http::StatusCode {
	fn from(status: StatusCode) -> Self {
		match status {
			StatusCode::Ok => http::StatusCode::OK,
			StatusCode::Created => http::StatusCode::CREATED,
			StatusCode::MalformedRequest => http::StatusCode::BAD_REQUEST,
			StatusCode::Unauthorized => http::StatusCode::UNAUTHORIZED,
			StatusCode::Forbidden => http::StatusCode::FORBIDDEN,
			StatusCode::NotFound => http::StatusCode::NOT_FOUND,
			StatusCode::MethodNotAllowed => {
				http::StatusCode::METHOD_NOT_ALLOWED
			}
			StatusCode::RequestTimeout => http::StatusCode::REQUEST_TIMEOUT,
			StatusCode::Conflict => http::StatusCode::CONFLICT,
			StatusCode::PayloadTooLarge => http::StatusCode::PAYLOAD_TOO_LARGE,
			StatusCode::RateLimitExceeded => {
				http::StatusCode::TOO_MANY_REQUESTS
			}
			StatusCode::PreconditionFailed => {
				http::StatusCode::PRECONDITION_FAILED
			}
			StatusCode::ImATeapot => http::StatusCode::IM_A_TEAPOT,
			StatusCode::MovedPermanently => http::StatusCode::MOVED_PERMANENTLY,
			StatusCode::TemporaryRedirect => {
				http::StatusCode::TEMPORARY_REDIRECT
			}
			StatusCode::InternalError => {
				http::StatusCode::INTERNAL_SERVER_ERROR
			}
			StatusCode::NotImplemented => http::StatusCode::NOT_IMPLEMENTED,
			StatusCode::ServiceUnavailable => {
				http::StatusCode::SERVICE_UNAVAILABLE
			}
			StatusCode::GatewayTimeout => http::StatusCode::GATEWAY_TIMEOUT,
			StatusCode::NoResponse => http::StatusCode::SERVICE_UNAVAILABLE,
			StatusCode::Cancelled => http::StatusCode::from_u16(499).unwrap(), // nginx convention
			StatusCode::AlreadyExists => http::StatusCode::CONFLICT,
			StatusCode::InvalidState => http::StatusCode::CONFLICT,
			StatusCode::ResourceExhausted => {
				http::StatusCode::SERVICE_UNAVAILABLE
			}
			StatusCode::DataLoss => http::StatusCode::INTERNAL_SERVER_ERROR,
			StatusCode::InvalidArgument => http::StatusCode::BAD_REQUEST,
			StatusCode::UpstreamFailure => http::StatusCode::BAD_GATEWAY,
			#[cfg(feature = "http")]
			StatusCode::Http(status) => status,
			StatusCode::Process(code) => {
				if code == 0 {
					http::StatusCode::OK
				} else {
					http::StatusCode::INTERNAL_SERVER_ERROR
				}
			}
		}
	}
}

impl From<StatusCode> for u8 {
	/// Converts to process exit code following BSD/UNIX conventions.
	///
	/// Exit codes follow sysexits.h conventions where applicable:
	/// - 0: Success
	/// - 1: General error
	/// - 64: Usage error (EX_USAGE)
	/// - 69: Service unavailable (EX_UNAVAILABLE)
	/// - 70: Internal software error (EX_SOFTWARE)
	/// - 73: Can't create (EX_CANTCREAT)
	/// - 74: I/O error (EX_IOERR)
	/// - 75: Temporary failure (EX_TEMPFAIL)
	/// - 77: Permission denied (EX_NOPERM)
	fn from(status: StatusCode) -> Self {
		match status {
			StatusCode::Ok | StatusCode::Created => 0,
			StatusCode::MalformedRequest | StatusCode::InvalidArgument => 64, // EX_USAGE
			StatusCode::Unauthorized | StatusCode::Forbidden => 77, // EX_NOPERM
			StatusCode::NotFound
			| StatusCode::MethodNotAllowed
			| StatusCode::Conflict
			| StatusCode::PayloadTooLarge
			| StatusCode::PreconditionFailed
			| StatusCode::ImATeapot
			| StatusCode::InvalidState
			| StatusCode::Cancelled
			| StatusCode::NotImplemented
			| StatusCode::UpstreamFailure => 1, // General error
			StatusCode::MovedPermanently | StatusCode::TemporaryRedirect => 0, // Redirects are success
			StatusCode::RequestTimeout
			| StatusCode::RateLimitExceeded
			| StatusCode::GatewayTimeout => 75, // EX_TEMPFAIL
			StatusCode::InternalError => 70, // EX_SOFTWARE
			StatusCode::ServiceUnavailable | StatusCode::NoResponse => 69, // EX_UNAVAILABLE
			StatusCode::AlreadyExists | StatusCode::ResourceExhausted => 73, // EX_CANTCREAT
			StatusCode::DataLoss => 74,                                      // EX_IOERR
			#[cfg(feature = "http")]
			StatusCode::Http(status) => {
				if status.is_success() {
					0
				} else {
					1
				}
			}
			StatusCode::Process(code) => code,
		}
	}
}

impl From<u8> for StatusCode {
	fn from(code: u8) -> Self {
		match code {
			0 => Self::Ok,
			64 => Self::MalformedRequest,
			69 => Self::ServiceUnavailable,
			70 => Self::InternalError,
			73 => Self::AlreadyExists,
			74 => Self::DataLoss,
			75 => Self::RequestTimeout,
			77 => Self::Unauthorized,
			_ => Self::Process(code),
		}
	}
}

#[cfg(all(feature = "http", feature = "serde"))]
fn serialize_http_status<S>(
	status: &http::StatusCode,
	serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	serializer.serialize_u16(status.as_u16())
}

#[cfg(all(feature = "http", feature = "serde"))]
fn deserialize_http_status<'de, D>(
	deserializer: D,
) -> Result<http::StatusCode, D::Error>
where
	D: serde::Deserializer<'de>,
{
	use serde::Deserialize;
	let code = u16::deserialize(deserializer)?;
	http::StatusCode::from_u16(code).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn is_ok() {
		StatusCode::Ok.is_ok().xpect_true();
		StatusCode::InternalError.is_ok().xpect_false();
		StatusCode::Process(0).is_ok().xpect_true();
		StatusCode::Process(1).is_ok().xpect_false();
	}

	#[test]
	#[cfg(feature = "http")]
	fn http_ok() {
		StatusCode::Http(http::StatusCode::OK).is_ok().xpect_true();
		StatusCode::Http(http::StatusCode::NOT_FOUND)
			.is_ok()
			.xpect_false();
	}

	#[test]
	fn client_error() {
		StatusCode::MalformedRequest.is_client_error().xpect_true();
		StatusCode::InternalError.is_client_error().xpect_false();
	}

	#[test]
	fn server_error() {
		StatusCode::InternalError.is_server_error().xpect_true();
		StatusCode::MalformedRequest.is_server_error().xpect_false();
	}

	#[test]
	fn exit_code_conversion() {
		let code: u8 = StatusCode::Ok.into();
		code.xpect_eq(0);

		let code: u8 = StatusCode::MalformedRequest.into();
		code.xpect_eq(64);

		let code: u8 = StatusCode::InternalError.into();
		code.xpect_eq(70);
	}

	#[test]
	fn from_exit_code() {
		StatusCode::from(0_u8).xpect_eq(StatusCode::Ok);
		StatusCode::from(64_u8).xpect_eq(StatusCode::MalformedRequest);
		StatusCode::from(70_u8).xpect_eq(StatusCode::InternalError);
		StatusCode::from(99_u8).xpect_eq(StatusCode::Process(99));
	}

	#[test]
	#[cfg(feature = "http")]
	fn from_http() {
		StatusCode::from(http::StatusCode::OK).xpect_eq(StatusCode::Ok);
		StatusCode::from(http::StatusCode::NOT_FOUND)
			.xpect_eq(StatusCode::NotFound);
		StatusCode::from(http::StatusCode::IM_A_TEAPOT)
			.xpect_eq(StatusCode::ImATeapot);
	}

	#[test]
	#[cfg(feature = "http")]
	fn to_http() {
		let status: http::StatusCode = StatusCode::Ok.into();
		status.xpect_eq(http::StatusCode::OK);

		let status: http::StatusCode = StatusCode::NotFound.into();
		status.xpect_eq(http::StatusCode::NOT_FOUND);

		let status: http::StatusCode = StatusCode::ImATeapot.into();
		status.xpect_eq(http::StatusCode::IM_A_TEAPOT);
	}

	#[test]
	fn default_is_ok() { StatusCode::default().xpect_eq(StatusCode::Ok); }

	#[test]
	fn to_exit_code() {
		StatusCode::Ok.to_exit_code().unwrap();
		StatusCode::InternalError
			.to_exit_code()
			.unwrap_err()
			.get()
			.xpect_eq(70);
		StatusCode::NotFound
			.to_exit_code()
			.unwrap_err()
			.get()
			.xpect_eq(1);
	}
}
