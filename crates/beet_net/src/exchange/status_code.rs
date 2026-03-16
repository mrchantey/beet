//! HTTP-based status codes for request/response exchanges.
//!
//! The [`StatusCode`] struct wraps a `u16` HTTP status code, providing
//! named constants and classification methods.
use beet_core::prelude::*;

/// An HTTP status code for request/response exchanges.
///
/// Wraps a `u16` value with named constants for common codes and
/// methods for classifying response types.
///
/// # Examples
///
/// ```rust
/// # use beet_net::prelude::*;
/// let ok = StatusCode::OK;
/// assert!(ok.is_ok());
///
/// let not_found = StatusCode::NOT_FOUND;
/// assert!(not_found.is_client_error());
///
/// // Create from raw code
/// let custom = StatusCode::new(201);
/// assert!(custom.is_ok());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StatusCode(u16);

impl Default for StatusCode {
	fn default() -> Self { Self::OK }
}

// 2xx Success
impl StatusCode {
	/// 200 OK
	pub const OK: StatusCode = StatusCode(200);
	/// 201 Created
	pub const CREATED: StatusCode = StatusCode(201);
	/// 204 No Content
	pub const NO_CONTENT: StatusCode = StatusCode(204);
}

// 3xx Redirection
impl StatusCode {
	/// 301 Moved Permanently
	pub const MOVED_PERMANENTLY: StatusCode = StatusCode(301);
	/// 302 Moved Permanently
	pub const FOUND: StatusCode = StatusCode(302);
	/// 303 See Other
	pub const SEE_OTHER: StatusCode = StatusCode(303);
	/// 307 Temporary Redirect
	pub const TEMPORARY_REDIRECT: StatusCode = StatusCode(307);
	/// 308 Temporary Redirect
	pub const PERMANENT_REDIRECT: StatusCode = StatusCode(308);
}

// 4xx Client Error
impl StatusCode {
	/// 400 Bad Request
	pub const BAD_REQUEST: StatusCode = StatusCode(400);
	/// 401 Unauthorized
	pub const UNAUTHORIZED: StatusCode = StatusCode(401);
	/// 403 Forbidden
	pub const FORBIDDEN: StatusCode = StatusCode(403);
	/// 404 Not Found
	pub const NOT_FOUND: StatusCode = StatusCode(404);
	/// 405 Method Not Allowed
	pub const METHOD_NOT_ALLOWED: StatusCode = StatusCode(405);
	/// 406 Not Acceptable
	pub const NOT_ACCEPTABLE: StatusCode = StatusCode(406);
	/// 408 Request Timeout
	pub const REQUEST_TIMEOUT: StatusCode = StatusCode(408);
	/// 409 Conflict
	pub const CONFLICT: StatusCode = StatusCode(409);
	/// 412 Precondition Failed
	pub const PRECONDITION_FAILED: StatusCode = StatusCode(412);
	/// 413 Payload Too Large
	pub const PAYLOAD_TOO_LARGE: StatusCode = StatusCode(413);
	/// 418 I'm a Teapot
	pub const IM_A_TEAPOT: StatusCode = StatusCode(418);
	/// 429 Too Many Requests
	pub const TOO_MANY_REQUESTS: StatusCode = StatusCode(429);
	/// 499 Client Closed Request (nginx convention)
	pub const CLIENT_CLOSED: StatusCode = StatusCode(499);
}

// 5xx Server Error
impl StatusCode {
	/// 500 Internal Server Error
	pub const INTERNAL_SERVER_ERROR: StatusCode = StatusCode(500);
	/// 501 Not Implemented
	pub const NOT_IMPLEMENTED: StatusCode = StatusCode(501);
	/// 502 Bad Gateway
	pub const BAD_GATEWAY: StatusCode = StatusCode(502);
	/// 503 Service Unavailable
	pub const SERVICE_UNAVAILABLE: StatusCode = StatusCode(503);
	/// 504 Gateway Timeout
	pub const GATEWAY_TIMEOUT: StatusCode = StatusCode(504);
}

impl StatusCode {
	/// Creates a [`StatusCode`] from a raw `u16` value.
	pub const fn new(code: u16) -> Self { Self(code) }

	/// Returns the `u16` value of this status code.
	pub const fn as_u16(&self) -> u16 { self.0 }

	/// Returns `true` if this is a 2xx or 3xx (success/redirect) status.
	pub const fn is_ok(&self) -> bool {
		self.is_success() || self.is_redirect()
	}

	/// Returns `true` if this is a 2xx success status.
	pub const fn is_success(&self) -> bool { self.0 >= 200 && self.0 < 300 }

	/// Returns `true` if this is a 3xx redirect status.
	pub const fn is_redirect(&self) -> bool { self.0 >= 300 && self.0 < 400 }

	/// Returns `true` if this is a 4xx client error status.
	pub const fn is_client_error(&self) -> bool {
		self.0 >= 400 && self.0 < 500
	}

	/// Returns `true` if this is a 5xx server error status.
	pub const fn is_server_error(&self) -> bool {
		self.0 >= 500 && self.0 < 600
	}

	/// Returns `true` if this status is a known redirect code (301, 302, 303, 307, 308).
	pub const fn is_redirect_location(&self) -> bool {
		matches!(self.0, 301 | 302 | 303 | 307 | 308)
	}

	/// Returns `true` if this status represents an error (4xx or 5xx).
	pub const fn is_err(&self) -> bool { !self.is_ok() }

	/// Converts status to exit code convention result.
	///
	/// Returns `Ok(())` for success, or `Err(NonZeroU8)` for errors.
	pub fn to_exit_code(&self) -> Result<(), std::num::NonZeroU8> {
		let code: u8 = self.to_process_exit_code();
		if code == 0 {
			Ok(())
		} else {
			Err(std::num::NonZeroU8::new(code).unwrap())
		}
	}

	/// Converts to a process exit code following BSD/UNIX conventions.
	///
	/// Exit codes follow sysexits.h conventions where applicable:
	/// - 0: Success (2xx, 3xx)
	/// - 1: General error
	/// - 64: Usage error, ie EX_USAGE (400, 405)
	/// - 69: Service unavailable, ie EX_UNAVAILABLE (503)
	/// - 70: Internal software error, ie EX_SOFTWARE (500)
	/// - 73: Can't create, ie EX_CANTCREAT (409)
	/// - 75: Temporary failure, ie EX_TEMPFAIL (408, 429, 504)
	/// - 77: Permission denied, ie EX_NOPERM (401, 403)
	pub fn to_process_exit_code(&self) -> u8 {
		match self.0 {
			200..=399 => 0,
			400 | 405 => 64,
			401 | 403 => 77,
			408 | 429 | 504 => 75,
			409 => 73,
			500 => 70,
			503 => 69,
			_ if self.is_ok() => 0,
			_ => 1,
		}
	}

	/// Returns the canonical reason phrase for this status code.
	pub fn message(&self) -> &'static str {
		match *self {
			Self::OK => "OK",
			Self::CREATED => "Created",
			Self::NO_CONTENT => "No Content",
			Self::MOVED_PERMANENTLY => "Moved Permanently",
			Self::TEMPORARY_REDIRECT => "Temporary Redirect",
			Self::BAD_REQUEST => "Bad Request",
			Self::UNAUTHORIZED => "Unauthorized",
			Self::FORBIDDEN => "Forbidden",
			Self::NOT_FOUND => "Not Found",
			Self::METHOD_NOT_ALLOWED => "Method Not Allowed",
			Self::NOT_ACCEPTABLE => "Not Acceptable",
			Self::REQUEST_TIMEOUT => "Request Timeout",
			Self::CONFLICT => "Conflict",
			Self::PRECONDITION_FAILED => "Precondition Failed",
			Self::PAYLOAD_TOO_LARGE => "Payload Too Large",
			Self::IM_A_TEAPOT => "I'm a Teapot",
			Self::TOO_MANY_REQUESTS => "Too Many Requests",
			Self::CLIENT_CLOSED => "Client Closed Request",
			Self::INTERNAL_SERVER_ERROR => "Internal Server Error",
			Self::NOT_IMPLEMENTED => "Not Implemented",
			Self::BAD_GATEWAY => "Bad Gateway",
			Self::SERVICE_UNAVAILABLE => "Service Unavailable",
			Self::GATEWAY_TIMEOUT => "Gateway Timeout",
			_ => "Unknown Status",
		}
	}
}

impl std::fmt::Display for StatusCode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{} {}", self.0, self.message())
	}
}

#[cfg(feature = "http")]
impl From<http::StatusCode> for StatusCode {
	fn from(status: http::StatusCode) -> Self { Self(status.as_u16()) }
}

#[cfg(feature = "http")]
impl From<StatusCode> for http::StatusCode {
	fn from(status: StatusCode) -> Self {
		http::StatusCode::from_u16(status.0)
			.unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR)
	}
}


impl From<u16> for StatusCode {
	fn from(code: u16) -> Self { Self(code) }
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn is_ok() {
		StatusCode::OK.is_ok().xpect_true();
		StatusCode::INTERNAL_SERVER_ERROR.is_ok().xpect_false();
		StatusCode::new(200).is_ok().xpect_true();
		StatusCode::new(500).is_ok().xpect_false();
	}

	#[test]
	#[cfg(feature = "http")]
	fn http_ok() {
		StatusCode::from(http::StatusCode::OK).is_ok().xpect_true();
		StatusCode::from(http::StatusCode::NOT_FOUND)
			.is_ok()
			.xpect_false();
	}

	#[test]
	fn client_error() {
		StatusCode::BAD_REQUEST.is_client_error().xpect_true();
		StatusCode::INTERNAL_SERVER_ERROR
			.is_client_error()
			.xpect_false();
	}

	#[test]
	fn server_error() {
		StatusCode::INTERNAL_SERVER_ERROR
			.is_server_error()
			.xpect_true();
		StatusCode::BAD_REQUEST.is_server_error().xpect_false();
	}

	#[test]
	#[cfg(feature = "http")]
	fn from_http() {
		StatusCode::from(http::StatusCode::OK).xpect_eq(StatusCode::OK);
		StatusCode::from(http::StatusCode::NOT_FOUND)
			.xpect_eq(StatusCode::NOT_FOUND);
		StatusCode::from(http::StatusCode::IM_A_TEAPOT)
			.xpect_eq(StatusCode::IM_A_TEAPOT);
	}

	#[test]
	#[cfg(feature = "http")]
	fn to_http() {
		let status: http::StatusCode = StatusCode::OK.into();
		status.xpect_eq(http::StatusCode::OK);

		let status: http::StatusCode = StatusCode::NOT_FOUND.into();
		status.xpect_eq(http::StatusCode::NOT_FOUND);

		let status: http::StatusCode = StatusCode::IM_A_TEAPOT.into();
		status.xpect_eq(http::StatusCode::IM_A_TEAPOT);
	}

	#[test]
	fn default_is_ok() { StatusCode::default().xpect_eq(StatusCode::OK); }

	#[test]
	fn to_exit_code() {
		StatusCode::OK.to_exit_code().unwrap();
		StatusCode::INTERNAL_SERVER_ERROR
			.to_exit_code()
			.unwrap_err()
			.get()
			.xpect_eq(70);
		StatusCode::NOT_FOUND
			.to_exit_code()
			.unwrap_err()
			.get()
			.xpect_eq(1);
	}

	#[test]
	fn message() {
		StatusCode::OK.message().xpect_eq("OK");
		StatusCode::NOT_FOUND.message().xpect_eq("Not Found");
		StatusCode::IM_A_TEAPOT.message().xpect_eq("I'm a Teapot");
		StatusCode::INTERNAL_SERVER_ERROR
			.message()
			.xpect_eq("Internal Server Error");
	}

	#[test]
	fn display() {
		format!("{}", StatusCode::OK).xpect_eq("200 OK");
		format!("{}", StatusCode::NOT_FOUND).xpect_eq("404 Not Found");
	}

	#[test]
	fn as_u16() {
		StatusCode::OK.as_u16().xpect_eq(200);
		StatusCode::NOT_FOUND.as_u16().xpect_eq(404);
		StatusCode::INTERNAL_SERVER_ERROR.as_u16().xpect_eq(500);
	}

	#[test]
	fn redirect_is_ok() {
		StatusCode::MOVED_PERMANENTLY.is_ok().xpect_true();
		StatusCode::TEMPORARY_REDIRECT.is_ok().xpect_true();
		StatusCode::MOVED_PERMANENTLY.is_redirect().xpect_true();
	}
}
