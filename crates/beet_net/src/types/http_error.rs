//! HTTP error type for request/response handling.
//!
//! This module provides [`HttpError`], an error type designed for HTTP-style
//! request handling that includes a status code and message, and the
//! [`FailureReport`] an internal error carries to whoever answers for it.
//!
//! # Security
//!
//! In release builds an internal error answers its client with an opaque
//! message, and its report is logged on the server instead.

use super::*;
use beet_core::prelude::*;
use bevy::ecs::system::RegisteredSystemError;
use bevy::ecs::system::RunSystemError;
use bevy::platform::sync::Arc;
use bevy::platform::sync::atomic::AtomicBool;
use bevy::platform::sync::atomic::Ordering;

/// A non-200 response from an HTTP request.
///
/// The message *will be* returned to the client, so ensure that no sensitive
/// information is included. A [`ParamError`] is a `400` carrying its message;
/// any other [`BevyError`] is an internal error, answered with an opaque
/// message in release builds and reported through its [`FailureReport`], see
/// [`from_opaque`](Self::from_opaque).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HttpError {
	/// The HTTP status code.
	pub status_code: StatusCode,
	/// The error message.
	pub message: String,
	/// An internal error's unredacted report, which never leaves the process.
	#[cfg_attr(feature = "serde", serde(skip))]
	pub report: Option<FailureReport>,
}

impl HttpError {
	/// Creates an [`HttpError`] with the given status code and an empty message.
	pub fn from_status(status_code: StatusCode) -> Self {
		Self::new(status_code, String::new())
	}

	/// Creates a new [`HttpError`] with the given status code and message.
	pub fn new(status_code: StatusCode, message: impl Into<String>) -> Self {
		Self {
			message: message.into(),
			status_code,
			report: None,
		}
	}

	/// Creates a 404 Not Found error.
	pub fn not_found() -> Self { Self::from_status(StatusCode::NOT_FOUND) }

	/// Creates a 400 Bad Request error with the given message.
	pub fn bad_request(message: impl Into<String>) -> Self {
		Self::new(StatusCode::BAD_REQUEST, message)
	}

	/// Creates a 500 Internal Server Error carrying `report` as its
	/// [`FailureReport`].
	///
	/// The message a client reads is the report in debug builds, with a
	/// reminder that a release build withholds it, and opaque in release.
	pub fn internal_error(report: impl Into<String>) -> Self {
		let report = FailureReport::new(report);
		#[cfg(debug_assertions)]
		let message = format!(
			"Internal Error: {report}\n\nThis error will *not* be returned to \
			the client in release builds.\n"
		);
		#[cfg(not(debug_assertions))]
		let message = StatusCode::INTERNAL_SERVER_ERROR.message().to_string();
		Self {
			status_code: StatusCode::INTERNAL_SERVER_ERROR,
			message,
			report: Some(report),
		}
	}

	/// Converts a [`BevyError`] into an [`HttpError`].
	///
	/// An [`HttpError`] is returned directly, and a [`ParamError`], a request
	/// param the caller got wrong, is a `400` carrying its message. Anything
	/// else is an [`internal_error`](Self::internal_error) reporting the
	/// error with its cause ([`ErrorReport`]).
	pub fn from_opaque(error: impl Into<BevyError>) -> Self {
		let error = error.into();
		if let Some(inner) = error.downcast_ref::<HttpError>() {
			inner.clone()
		} else if let Some(param) = error.downcast_ref::<ParamError>() {
			Self::bad_request(param.to_string())
		} else {
			Self::internal_error(ErrorReport::new(&error).to_string())
		}
	}
}

/// An internal error's unredacted report, riding its `500` (an [`HttpError`],
/// then its [`Response`]'s parts) to whoever answers for the failure, and
/// reported exactly once.
///
/// Whose failure it is depends on who asked. A local command's caller is the
/// operator, so the CLI [`claim`](Self::claim)s the report and prints it as
/// the command's one failure report. A remote client gets the message
/// instead, redacted in release, so nothing claims the report and it is
/// logged at `ERROR` when its last copy drops, ie once the response is sent.
#[derive(Clone)]
pub struct FailureReport(Arc<FailureReportInner>);

/// The shared half of a [`FailureReport`], logging an unclaimed report on drop.
struct FailureReportInner {
	text: String,
	claimed: AtomicBool,
}

impl FailureReport {
	/// A report of `text`, logged on drop unless claimed.
	pub fn new(text: impl Into<String>) -> Self {
		Self(Arc::new(FailureReportInner {
			text: text.into(),
			claimed: AtomicBool::new(false),
		}))
	}

	/// Answer for this failure: the report the first time, after which no
	/// copy logs it, and `None` once claimed.
	pub fn claim(&self) -> Option<&str> {
		(!self.0.claimed.swap(true, Ordering::AcqRel))
			.then_some(self.0.text.as_str())
	}
}

impl Drop for FailureReportInner {
	fn drop(&mut self) {
		if !*self.claimed.get_mut() {
			error!("Internal Error: {}", self.text);
		}
	}
}

impl core::fmt::Display for FailureReport {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.write_str(&self.0.text)
	}
}

impl core::fmt::Debug for FailureReport {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_tuple("FailureReport").field(&self.0.text).finish()
	}
}

impl core::error::Error for HttpError {}

impl core::fmt::Display for HttpError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		if self.message.is_empty() {
			write!(f, "{}", self.status_code)
		} else {
			write!(f, "{}: {}", self.status_code, self.message)
		}
	}
}
impl From<BevyError> for HttpError {
	fn from(err: BevyError) -> HttpError { Self::from_opaque(err) }
}
impl From<RunSystemError> for HttpError {
	fn from(err: RunSystemError) -> HttpError {
		Self::internal_error(err.to_string())
	}
}
impl<In, Out> From<RegisteredSystemError<In, Out>> for HttpError
where
	In: 'static + SystemInput,
	Out: 'static,
{
	fn from(err: RegisteredSystemError<In, Out>) -> HttpError {
		Self::from_opaque(err)
	}
}

#[cfg(feature = "json")]
impl From<serde_json::Error> for HttpError {
	fn from(err: serde_json::Error) -> HttpError { Self::from_opaque(err) }
}

#[cfg(all(feature = "serde", feature = "std"))]
impl From<serde_urlencoded::de::Error> for HttpError {
	fn from(err: serde_urlencoded::de::Error) -> HttpError {
		Self::from_opaque(err)
	}
}

impl From<HttpError> for Response {
	fn from(error: HttpError) -> Response {
		let mut response = if error.message.is_empty() {
			Response::from_status(error.status_code)
		} else {
			Response::from_status_body(
				error.status_code,
				error.message.as_bytes(),
				MediaType::Text,
			)
		};
		response.parts.report = error.report;
		response
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[derive(Reflect)]
	#[allow(dead_code)]
	struct ServeParams {
		port: u16,
	}

	/// The status of the response a failed parse of `args` answers.
	fn parse_status(args: &str) -> StatusCode {
		Request::from_cli_str(args)
			.parse_params::<ServeParams>()
			.err()
			.unwrap()
			.xmap(Response::from)
			.status()
	}

	/// A param the caller got wrong, missing or malformed, is their error, so
	/// a CLI exits with its usage code rather than an internal error's.
	#[beet_core::test]
	fn param_errors_are_bad_requests() {
		parse_status("").xpect_eq(StatusCode::BAD_REQUEST);
		parse_status("--port=abc").xpect_eq(StatusCode::BAD_REQUEST);
		StatusCode::BAD_REQUEST.to_process_exit_code().xpect_eq(64);
		HttpError::from_opaque(bevyhow!("disk on fire"))
			.status_code
			.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
	}

	/// A writer collecting what a subscriber writes.
	#[derive(Default, Clone)]
	struct Capture(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

	impl std::io::Write for Capture {
		fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
			self.0.lock().unwrap().extend_from_slice(data);
			Ok(data.len())
		}
		fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
	}

	/// What `func` logs on this thread.
	fn logged(func: impl FnOnce()) -> String {
		let capture = Capture::default();
		let writer = capture.clone();
		let subscriber = bevy::log::tracing_subscriber::fmt()
			.with_writer(move || writer.clone())
			.with_ansi(false)
			// wasm has no system clock
			.without_time()
			.finish();
		bevy::log::tracing::subscriber::with_default(subscriber, func);
		String::from_utf8(capture.0.lock().unwrap().clone()).unwrap()
	}

	/// The http path: a remote client reads the message (the report and a
	/// release reminder in a debug build, opaque in release), and the report
	/// is logged once, when the last copy of the response drops.
	#[beet_core::test]
	fn an_unclaimed_report_logs_once() {
		let error = HttpError::from_opaque(bevyhow!("disk on fire"));
		#[cfg(debug_assertions)]
		error
			.message
			.clone()
			.xpect_contains("disk on fire")
			.xpect_contains("will *not* be returned to the client")
			.xpect_ends_with("\n");
		logged(move || {
			let response = Response::from(error.clone());
			drop(error);
			drop(response);
		})
		.matches("Internal Error: disk on fire")
		.count()
		.xpect_eq(1);
	}

	/// The CLI path: the report is claimed exactly once, and a claimed report
	/// logs nothing, since its claimant prints it.
	#[beet_core::test]
	fn a_claimed_report_is_not_logged() {
		logged(|| {
			let response = Response::from(HttpError::from_opaque(bevyhow!(
				"disk on fire"
			)));
			let report = response.parts.report.clone().unwrap();
			report.claim().unwrap().xpect_starts_with("disk on fire");
			report.claim().xpect_none();
		})
		.xpect_eq("");
	}
}
