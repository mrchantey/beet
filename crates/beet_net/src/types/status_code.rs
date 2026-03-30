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

impl StatusCode {
	/// Creates a [`StatusCode`] from a raw `u16` value.
	pub const fn new(code: u16) -> Self { Self(code) }

	/// Returns the `u16` value of this status code.
	pub const fn as_u16(&self) -> u16 { self.0 }

	/// Returns `true` if this is a 2xx or 3xx (success/redirect) status.
	pub const fn is_ok(&self) -> bool {
		self.is_success() || self.is_redirect()
	}

	/// Returns `true` if this is a 1xx informational status.
	pub const fn is_informational(&self) -> bool {
		self.0 >= 100 && self.0 < 200
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
		match self.0 {
			100 => "Continue",
			101 => "Switching Protocols",
			102 => "Processing",
			103 => "Early Hints",
			200 => "OK",
			201 => "Created",
			202 => "Accepted",
			203 => "Non-Authoritative Information",
			204 => "No Content",
			205 => "Reset Content",
			206 => "Partial Content",
			207 => "Multi-Status",
			208 => "Already Reported",
			226 => "IM Used",
			300 => "Multiple Choices",
			301 => "Moved Permanently",
			302 => "Found",
			303 => "See Other",
			304 => "Not Modified",
			305 => "Use Proxy",
			307 => "Temporary Redirect",
			308 => "Permanent Redirect",
			400 => "Bad Request",
			401 => "Unauthorized",
			402 => "Payment Required",
			403 => "Forbidden",
			404 => "Not Found",
			405 => "Method Not Allowed",
			406 => "Not Acceptable",
			407 => "Proxy Authentication Required",
			408 => "Request Timeout",
			409 => "Conflict",
			410 => "Gone",
			411 => "Length Required",
			412 => "Precondition Failed",
			413 => "Content Too Large",
			414 => "URI Too Long",
			415 => "Unsupported Media Type",
			416 => "Range Not Satisfiable",
			417 => "Expectation Failed",
			418 => "I'm a Teapot",
			421 => "Misdirected Request",
			422 => "Unprocessable Content",
			423 => "Locked",
			424 => "Failed Dependency",
			425 => "Too Early",
			426 => "Upgrade Required",
			428 => "Precondition Required",
			429 => "Too Many Requests",
			431 => "Request Header Fields Too Large",
			451 => "Unavailable For Legal Reasons",
			499 => "Client Closed Request",
			500 => "Internal Server Error",
			501 => "Not Implemented",
			502 => "Bad Gateway",
			503 => "Service Unavailable",
			504 => "Gateway Timeout",
			505 => "HTTP Version Not Supported",
			506 => "Variant Also Negotiates",
			507 => "Insufficient Storage",
			508 => "Loop Detected",
			510 => "Not Extended",
			511 => "Network Authentication Required",
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


// 1xx Informational
impl StatusCode {
	/// # 100 Continue
	/// The client should continue the request or ignore this response if the request is already finished.
	pub const CONTINUE: StatusCode = StatusCode(100);

	/// # 101 Switching Protocols
	/// Sent in response to an `Upgrade` request header, indicating the protocol the server is switching to.
	pub const SWITCHING_PROTOCOLS: StatusCode = StatusCode(101);

	/// # 102 Processing (WebDAV) — Deprecated
	/// Indicates a request has been received but no status was available at the time of the response.
	pub const PROCESSING: StatusCode = StatusCode(102);

	/// # 103 Early Hints
	/// Primarily used with the `Link` header, letting the user agent start preloading resources while the server prepares a response.
	pub const EARLY_HINTS: StatusCode = StatusCode(103);
}

// 2xx Success
impl StatusCode {
	/// # 200 OK
	/// The request succeeded. The meaning of "success" depends on the HTTP method used:
	/// - `GET`: The resource has been fetched and transmitted in the message body.
	/// - `HEAD`: Representation headers are included in the response without any message body.
	/// - `PUT` or `POST`: The resource describing the result of the action is transmitted in the message body.
	/// - `TRACE`: The message body contains the request as received by the server.
	pub const OK: StatusCode = StatusCode(200);

	/// # 201 Created
	/// The request succeeded and a new resource was created. Typically the response to POST or some PUT requests.
	pub const CREATED: StatusCode = StatusCode(201);

	/// # 202 Accepted
	/// The request has been received but not yet acted upon. Intended for cases where another process or server handles the request.
	pub const ACCEPTED: StatusCode = StatusCode(202);

	/// # 203 Non-Authoritative Information
	/// The returned metadata is not exactly the same as available from the origin server, but is collected from a local or third-party copy.
	pub const NON_AUTHORITATIVE_INFORMATION: StatusCode = StatusCode(203);

	/// # 204 No Content
	/// There is no content to send for this request, but the headers may be useful.
	pub const NO_CONTENT: StatusCode = StatusCode(204);

	/// # 205 Reset Content
	/// Tells the user agent to reset the document which sent this request.
	pub const RESET_CONTENT: StatusCode = StatusCode(205);

	/// # 206 Partial Content
	/// Used in response to a range request when the client has requested a part or parts of a resource.
	pub const PARTIAL_CONTENT: StatusCode = StatusCode(206);

	/// # 207 Multi-Status (WebDAV)
	/// Conveys information about multiple resources, for situations where multiple status codes might be appropriate.
	pub const MULTI_STATUS: StatusCode = StatusCode(207);

	/// # 208 Already Reported (WebDAV)
	/// Used inside a `<dav:propstat>` response element to avoid repeatedly enumerating the internal members of multiple bindings to the same collection.
	pub const ALREADY_REPORTED: StatusCode = StatusCode(208);

	/// # 226 IM Used (HTTP Delta encoding)
	/// The server has fulfilled a GET request, and the response is a result of one or more instance-manipulations applied to the current instance.
	pub const IM_USED: StatusCode = StatusCode(226);
}

// 3xx Redirection
impl StatusCode {
	/// # 300 Multiple Choices
	/// The request has more than one possible response; the user agent or user should choose one.
	pub const MULTIPLE_CHOICES: StatusCode = StatusCode(300);

	/// # 301 Moved Permanently
	/// The URL of the requested resource has been changed permanently. The new URL is given in the response.
	pub const MOVED_PERMANENTLY: StatusCode = StatusCode(301);

	/// # 302 Found
	/// The URI of the requested resource has been changed temporarily. The same URI should be used by the client in future requests.
	pub const FOUND: StatusCode = StatusCode(302);

	/// # 303 See Other
	/// The server sent this response to direct the client to get the requested resource at another URI with a GET request.
	pub const SEE_OTHER: StatusCode = StatusCode(303);

	/// # 304 Not Modified
	/// Used for caching. Tells the client the response has not been modified, so the client can continue to use the same cached version.
	pub const NOT_MODIFIED: StatusCode = StatusCode(304);

	/// # 305 Use Proxy — Deprecated
	/// Indicated that the requested response must be accessed by a proxy. Deprecated due to security concerns.
	pub const USE_PROXY: StatusCode = StatusCode(305);

	/// # 307 Temporary Redirect
	/// The requested resource is at another URI using the same HTTP method. The user agent must not change the method used.
	pub const TEMPORARY_REDIRECT: StatusCode = StatusCode(307);

	/// # 308 Permanent Redirect
	/// The resource is now permanently located at another URI. The user agent must not change the HTTP method used.
	pub const PERMANENT_REDIRECT: StatusCode = StatusCode(308);
}

// 4xx Client Error
impl StatusCode {
	/// # 400 Bad Request
	/// The server cannot process the request due to a client error, ie malformed syntax or invalid framing.
	pub const BAD_REQUEST: StatusCode = StatusCode(400);

	/// # 401 Unauthorized
	/// Semantically means "unauthenticated": the client must authenticate itself to get the requested response.
	pub const UNAUTHORIZED: StatusCode = StatusCode(401);

	/// # 402 Payment Required
	/// Reserved for digital payment systems; rarely used and no standard convention exists.
	pub const PAYMENT_REQUIRED: StatusCode = StatusCode(402);

	/// # 403 Forbidden
	/// The client does not have access rights to the content. Unlike 401, the client's identity is known to the server.
	pub const FORBIDDEN: StatusCode = StatusCode(403);

	/// # 404 Not Found
	/// The server cannot find the requested resource. Probably the most well-known status code on the web.
	pub const NOT_FOUND: StatusCode = StatusCode(404);

	/// # 405 Method Not Allowed
	/// The request method is known by the server but not supported by the target resource.
	pub const METHOD_NOT_ALLOWED: StatusCode = StatusCode(405);

	/// # 406 Not Acceptable
	/// After server-driven content negotiation, no content conforming to the user agent's criteria was found.
	pub const NOT_ACCEPTABLE: StatusCode = StatusCode(406);

	/// # 407 Proxy Authentication Required
	/// Similar to 401 Unauthorized, but authentication must be performed by a proxy.
	pub const PROXY_AUTHENTICATION_REQUIRED: StatusCode = StatusCode(407);

	/// # 408 Request Timeout
	/// Sent on an idle connection by some servers; the server would like to shut down this unused connection.
	pub const REQUEST_TIMEOUT: StatusCode = StatusCode(408);

	/// # 409 Conflict
	/// The request conflicts with the current state of the server.
	pub const CONFLICT: StatusCode = StatusCode(409);

	/// # 410 Gone
	/// The requested content has been permanently deleted from the server with no forwarding address.
	pub const GONE: StatusCode = StatusCode(410);

	/// # 411 Length Required
	/// The server rejected the request because the `Content-Length` header field is not defined and the server requires it.
	pub const LENGTH_REQUIRED: StatusCode = StatusCode(411);

	/// # 412 Precondition Failed
	/// The client has indicated preconditions in its headers which the server does not meet.
	pub const PRECONDITION_FAILED: StatusCode = StatusCode(412);

	/// # 413 Content Too Large
	/// The request body is larger than limits defined by the server.
	pub const CONTENT_TOO_LARGE: StatusCode = StatusCode(413);

	/// # 414 URI Too Long
	/// The URI requested by the client is longer than the server is willing to interpret.
	pub const URI_TOO_LONG: StatusCode = StatusCode(414);

	/// # 415 Unsupported Media Type
	/// The media format of the requested data is not supported by the server.
	pub const UNSUPPORTED_MEDIA_TYPE: StatusCode = StatusCode(415);

	/// # 416 Range Not Satisfiable
	/// The ranges specified by the `Range` header field cannot be fulfilled; the range may be outside the target resource's data.
	pub const RANGE_NOT_SATISFIABLE: StatusCode = StatusCode(416);

	/// # 417 Expectation Failed
	/// The expectation indicated by the `Expect` request header field cannot be met by the server.
	pub const EXPECTATION_FAILED: StatusCode = StatusCode(417);

	/// # 418 I'm a Teapot
	/// The server refuses the attempt to brew coffee with a teapot.
	pub const IM_A_TEAPOT: StatusCode = StatusCode(418);

	/// # 421 Misdirected Request
	/// The request was directed at a server not able to produce a response for the given scheme and authority.
	pub const MISDIRECTED_REQUEST: StatusCode = StatusCode(421);

	/// # 422 Unprocessable Content (WebDAV)
	/// The request was well-formed but could not be followed due to semantic errors.
	pub const UNPROCESSABLE_CONTENT: StatusCode = StatusCode(422);

	/// # 423 Locked (WebDAV)
	/// The resource being accessed is locked.
	pub const LOCKED: StatusCode = StatusCode(423);

	/// # 424 Failed Dependency (WebDAV)
	/// The request failed due to failure of a previous request.
	pub const FAILED_DEPENDENCY: StatusCode = StatusCode(424);

	/// # 425 Too Early (Experimental)
	/// The server is unwilling to risk processing a request that might be replayed.
	pub const TOO_EARLY: StatusCode = StatusCode(425);

	/// # 426 Upgrade Required
	/// The server refuses to perform the request using the current protocol but may do so after the client upgrades.
	pub const UPGRADE_REQUIRED: StatusCode = StatusCode(426);

	/// # 428 Precondition Required
	/// The origin server requires the request to be conditional, to prevent the "lost update" problem.
	pub const PRECONDITION_REQUIRED: StatusCode = StatusCode(428);

	/// # 429 Too Many Requests
	/// The user has sent too many requests in a given amount of time (rate limiting).
	pub const TOO_MANY_REQUESTS: StatusCode = StatusCode(429);

	/// # 431 Request Header Fields Too Large
	/// The server is unwilling to process the request because its header fields are too large.
	pub const REQUEST_HEADER_FIELDS_TOO_LARGE: StatusCode = StatusCode(431);

	/// # 451 Unavailable For Legal Reasons
	/// The user agent requested a resource that cannot legally be provided, such as a page censored by a government.
	pub const UNAVAILABLE_FOR_LEGAL_REASONS: StatusCode = StatusCode(451);

	/// # 499 Client Closed Request (nginx convention)
	/// The client closed the connection before the server could respond. Not part of the HTTP standard.
	pub const CLIENT_CLOSED: StatusCode = StatusCode(499);
}

// 5xx Server Error
impl StatusCode {
	/// # 500 Internal Server Error
	/// The server encountered a situation it does not know how to handle.
	pub const INTERNAL_SERVER_ERROR: StatusCode = StatusCode(500);

	/// # 501 Not Implemented
	/// The request method is not supported by the server and cannot be handled.
	pub const NOT_IMPLEMENTED: StatusCode = StatusCode(501);

	/// # 502 Bad Gateway
	/// The server, acting as a gateway, received an invalid response from an upstream server.
	pub const BAD_GATEWAY: StatusCode = StatusCode(502);

	/// # 503 Service Unavailable
	/// The server is not ready to handle the request, ie due to maintenance or overload.
	pub const SERVICE_UNAVAILABLE: StatusCode = StatusCode(503);

	/// # 504 Gateway Timeout
	/// The server, acting as a gateway, cannot get a response in time.
	pub const GATEWAY_TIMEOUT: StatusCode = StatusCode(504);

	/// # 505 HTTP Version Not Supported
	/// The HTTP version used in the request is not supported by the server.
	pub const HTTP_VERSION_NOT_SUPPORTED: StatusCode = StatusCode(505);

	/// # 506 Variant Also Negotiates
	/// The server has an internal configuration error: the chosen variant is itself configured to engage in content negotiation, causing circular references.
	pub const VARIANT_ALSO_NEGOTIATES: StatusCode = StatusCode(506);

	/// # 507 Insufficient Storage (WebDAV)
	/// The server is unable to store the representation needed to successfully complete the request.
	pub const INSUFFICIENT_STORAGE: StatusCode = StatusCode(507);

	/// # 508 Loop Detected (WebDAV)
	/// The server detected an infinite loop while processing the request.
	pub const LOOP_DETECTED: StatusCode = StatusCode(508);

	/// # 510 Not Extended
	/// The client request declares an HTTP Extension (RFC 2774) that should be used to process the request, but the extension is not supported.
	pub const NOT_EXTENDED: StatusCode = StatusCode(510);

	/// # 511 Network Authentication Required
	/// The client needs to authenticate to gain network access.
	pub const NETWORK_AUTHENTICATION_REQUIRED: StatusCode = StatusCode(511);
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
	fn informational() {
		StatusCode::CONTINUE.is_informational().xpect_true();
		StatusCode::EARLY_HINTS.is_informational().xpect_true();
		StatusCode::OK.is_informational().xpect_false();
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
		StatusCode::CONTINUE.message().xpect_eq("Continue");
		StatusCode::ACCEPTED.message().xpect_eq("Accepted");
		StatusCode::GONE.message().xpect_eq("Gone");
		StatusCode::LOOP_DETECTED
			.message()
			.xpect_eq("Loop Detected");
	}

	#[test]
	fn display() {
		format!("{}", StatusCode::OK).xpect_eq("200 OK");
		format!("{}", StatusCode::NOT_FOUND).xpect_eq("404 Not Found");
		format!("{}", StatusCode::CONTINUE).xpect_eq("100 Continue");
	}

	#[test]
	fn as_u16() {
		StatusCode::OK.as_u16().xpect_eq(200);
		StatusCode::NOT_FOUND.as_u16().xpect_eq(404);
		StatusCode::INTERNAL_SERVER_ERROR.as_u16().xpect_eq(500);
		StatusCode::CONTINUE.as_u16().xpect_eq(100);
		StatusCode::LOOP_DETECTED.as_u16().xpect_eq(508);
	}

	#[test]
	fn redirect_is_ok() {
		StatusCode::MOVED_PERMANENTLY.is_ok().xpect_true();
		StatusCode::TEMPORARY_REDIRECT.is_ok().xpect_true();
		StatusCode::PERMANENT_REDIRECT.is_ok().xpect_true();
		StatusCode::MOVED_PERMANENTLY.is_redirect().xpect_true();
	}
}
