#[allow(unused)]
use crate::prelude::*;
#[allow(unused)]
use anyhow::Result;

/// A sweet error is the panic payload emitted by matchers:
/// ```should_panic
/// # use sweet::prelude::*;
/// // this will panic with a MatcherErr
/// expect(true).to_be_false();
/// ```
/// The magic of sweet matchers lies here in the bactrace building.
/// It is absolutely critical to respect call site depth when building
/// a SweetError, or the emitted frame will be at the wrong depth.
///
/// # Important
///
/// Compiler optimizations can cause the backtrace to be incorrect.
/// if optimizing dev or test profiles for some packages, at least the
/// following must be unoptimized:
///
/// ```toml
/// # Cargo.toml
/// [profile.test]
/// opt-level = 0
/// [profile.test.package.sweet]
/// opt-level = 0
/// ```
#[derive(Debug, Clone)]
pub struct SweetError {
	pub message: String,
	#[cfg(not(target_arch = "wasm32"))]
	assertion_depth: usize,
	#[cfg(not(target_arch = "wasm32"))]
	backtrace: backtrace::Backtrace,
}

impl std::error::Error for SweetError {}

impl SweetError {
	/// callsite of a users expect, ie
	/// ```
	/// # use sweet::prelude::*;
	/// expect(true).to_be_true();
	/// ```
	pub const BACKTRACE_LEVEL_5: usize = 5;
	/// callsite of Matcher::to_be, ie [Matcher::to_be_true]
	pub const BACKTRACE_LEVEL_4: usize = 4;
	/// callsite of Matcher::assert, ie [Matcher::assert_equal]
	pub const BACKTRACE_LEVEL_3: usize = 3;
	/// callsite of Matcher::panic_if, ie [Matcher::panic_if_negated]
	pub const BACKTRACE_LEVEL_2: usize = 2;
	/// callsite of [SweetError::panic]
	pub const BACKTRACE_LEVEL_1: usize = 1;
	/// callsite of [SweetError::new]
	pub const BACKTRACE_LEVEL_0: usize = 0;


	#[allow(unused)]
	pub fn new(message: impl Into<String>, mut assertion_depth: usize) -> Self {
		// not sure why the windows backtrace is so much deeper
		#[cfg(target_os = "windows")]
		{
			assertion_depth += 4;
		}
		#[cfg(target_arch = "wasm32")]
		return Self {
			message: message.into(),
		};
		#[cfg(not(target_arch = "wasm32"))]
		return Self {
			message: message.into(),
			backtrace: backtrace::Backtrace::new_unresolved(),
			assertion_depth,
		};
	}

	/// Must be called at [`SweetError::BACKTRACE_LEVEL_1`]
	/// This only works with both `sweet` and the crate
	/// entirely unoptimized.
	pub fn panic(message: impl Into<String>) -> ! {
		let backtrace_level = 5;

		std::panic::panic_any(Self::new(message, backtrace_level));
	}
}

impl std::fmt::Display for SweetError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.write_str(&self.message)
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl SweetError {
	fn assertion_location(&self) -> Result<BacktraceLocation> {
		let mut assertion_depth = self.assertion_depth;
		// how to test this?
		loop {
			if let Some(frame) = self.backtrace.frames().get(assertion_depth) {
				let loc = BacktraceLocation::from_unresolved_frame(frame)?;
				// skip frames in the cargo registry
				// TODO maybe expose this as a filter func?
				if !loc
					.cwd_path
					.to_string_lossy()
					.contains(".cargo/registry/src/index.crates.io-")
				{
					break Ok(loc);
				}
				assertion_depth += 1;
			} else {
				anyhow::bail!(
					"Failed to find backtrace frame at depth {}",
					assertion_depth
				);
			}
		}
	}

	pub fn backtrace_str(&self) -> Result<String> {
		self.assertion_location()?.file_context()
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	#[cfg(not(target_arch = "wasm32"))]
	fn works() {
		let err = SweetError::new("expected bar", 1);
		let msg = err.backtrace_str().unwrap();
		let lines = msg.lines().collect::<Vec<_>>();

		expect(lines[BacktraceLocation::LINE_CONTEXT_SIZE])
			.to_contain("let err = SweetError::new");
	}
	#[test]
	#[ignore = "use for visual testing"]
	fn panics() { std::panic::panic_any(SweetError::new("expected bar", 1)); }
}
