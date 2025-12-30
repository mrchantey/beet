//! Minimal error type for sweet matchers.

/// A sweet error is the panic payload emitted by matchers.
///
/// # Example
/// ```should_panic
/// # use sweet::prelude::*;
/// // this will panic with a formatted message
/// true.xpect_false();
/// ```
#[derive(Debug, Clone)]
pub struct SweetError {
	pub message: String,
}

impl std::error::Error for SweetError {}

impl SweetError {
	pub fn new(message: impl Into<String>) -> Self {
		Self {
			message: message.into(),
		}
	}
}

impl std::fmt::Display for SweetError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.write_str(&self.message)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn creates_error() {
		let err = SweetError::new("expected bar");
		err.message.xpect_eq("expected bar");
	}
}
