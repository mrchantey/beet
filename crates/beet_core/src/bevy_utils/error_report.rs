//! The final rendering of an error, where it ends a request or a process.

use crate::prelude::*;
use alloc::string::String;
use alloc::string::ToString;
use bevy::prelude::BevyError;

/// An error as it is finally reported, a failed command's stderr or a 500's
/// log: its message, then whatever cause the message leaves out.
///
/// A [`BevyError`] keeps the error it erased private, exposing only its
/// `Display`, which is the outermost message alone (the AWS SDK's `dispatch
/// failure`), and its `Debug`, which for a derived error type nests every
/// `source` field down the chain. So the cause of an erased error is its
/// `Debug`, reported when it says more than the message does; a beet message
/// (`bevyhow!`) already says everything and reports as itself.
///
/// ```
/// # use beet_core::prelude::*;
/// let err: BevyError = bevyhow!("disk on fire");
/// ErrorReport::new(&err).to_string().xpect_starts_with("disk on fire");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorReport(String);

impl ErrorReport {
	/// The report of `error`, see [`ErrorReport`].
	pub fn new(error: &BevyError) -> Self {
		let message = error.to_string();
		let debug = alloc::format!("{error:?}");
		// the erased error's own `Debug` leads, one line for a derived type
		let cause = debug.lines().next().unwrap_or_default();
		match Self::normalize(&message).contains(&Self::normalize(cause)) {
			true => Self(message.trim_end().to_string()),
			false => Self(alloc::format!(
				"{}\n  caused by: {cause}",
				message.trim_end()
			)),
		}
	}

	/// `text` with every whitespace run as one space, so a `Debug` that
	/// indents differently from its `Display` still reads as the same words.
	fn normalize(text: &str) -> String {
		text.split_whitespace().collect::<Vec<_>>().join(" ")
	}
}

impl core::fmt::Display for ErrorReport {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.write_str(&self.0)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A transport failure hiding its cause behind a terse message, the shape
	/// of the AWS SDK's `SdkError::DispatchFailure`.
	#[derive(Debug)]
	struct DispatchFailure {
		source: ConnectorError,
	}

	impl core::fmt::Display for DispatchFailure {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			f.write_str("dispatch failure")
		}
	}

	impl core::error::Error for DispatchFailure {
		fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
			Some(&self.source)
		}
	}

	#[derive(Debug)]
	struct ConnectorError {
		reason: &'static str,
	}

	impl core::fmt::Display for ConnectorError {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			f.write_str(self.reason)
		}
	}

	impl core::error::Error for ConnectorError {}

	#[crate::test]
	fn reports_the_nested_cause() {
		let err = BevyError::from(DispatchFailure {
			source: ConnectorError {
				reason: "no providers in chain provided credentials",
			},
		});
		ErrorReport::new(&err)
			.to_string()
			.xpect_starts_with("dispatch failure\n  caused by: ")
			.xpect_contains("ConnectorError")
			.xpect_contains("no providers in chain provided credentials");
	}

	/// A beet message already says everything, so it reports as itself.
	#[crate::test]
	fn a_message_reports_as_itself() {
		let err = bevyhow!("disk on fire");
		ErrorReport::new(&err)
			.to_string()
			.xpect_starts_with("disk on fire\n  at ")
			.xnot()
			.xpect_contains("caused by");
		let cloned = BevyError::from(CloneError::new(bevyhow!("disk on fire")));
		ErrorReport::new(&cloned)
			.to_string()
			.xnot()
			.xpect_contains("caused by");
	}
}
