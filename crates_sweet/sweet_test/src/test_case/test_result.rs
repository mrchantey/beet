use crate::prelude::*;
use colorize::*;
use std::panic::PanicHookInfo;
use test::ShouldPanic;
use test::TestDesc;

/// a method for sending test descriptions with outputs
/// This implementation may change to be more restricted
/// to reduce clone cost
#[derive(Debug)]
pub struct TestDescAndResult {
	pub desc: TestDesc,
	pub result: TestResult,
}


pub struct TestDescAndFuture {
	pub desc: TestDesc,
	pub fut: SweetFutFunc,
}

impl TestDescAndFuture {
	pub fn new(desc: TestDesc, fut: SweetFutFunc) -> Self { Self { desc, fut } }
}

impl TestDescAndResult {
	pub fn new(desc: TestDesc, result: TestResult) -> Self {
		Self { desc, result }
	}
}

pub enum TestResultOrFut {
	Result(TestDescAndResult),
	Fut(TestDescAndFuture),
}


#[derive(Debug, Clone, PartialEq)]
pub enum TestResult {
	Pass,
	Fail(String),
	Ignore(Option<String>),
}

impl TestResult {
	pub fn status_prefix(&self) -> String {
		match self {
			TestResult::Pass => " PASS ".black().bold().greenb(),
			TestResult::Fail(_) => " FAIL ".black().bold().redb(),
			TestResult::Ignore(_) => " SKIP ".black().bold().yellowb(),
		}
	}

	/// The message to display for the test result
	/// If pass this is an empty string
	pub fn message(&self) -> String {
		match self {
			TestResult::Pass => String::new(),
			TestResult::Fail(msg) => format!("\n\n{}", msg),
			TestResult::Ignore(Some(msg)) => format!("\t{}", msg).yellow(),
			TestResult::Ignore(None) => String::new(),
		}
	}

	/// This must be called directly from the panic hook
	/// or else the bactrace frame will be off
	pub fn from_panic(info: &PanicHookInfo, desc: &TestDesc) -> Self {
		match &desc.should_panic {
			ShouldPanic::Yes => TestResult::Pass,
			ShouldPanic::YesWithMessage(_) => TestResult::Pass,
			ShouldPanic::No => {
				let panic_ctx = || {
					BacktraceLocation::from_panic_info(info, desc)
						.file_context()
				};
				let (payload, bt) =
					if let Some(str) = info.payload().downcast_ref::<&str>() {
						(str.to_string(), panic_ctx())
					} else if let Some(str) =
						info.payload().downcast_ref::<String>()
					{
						(str.clone(), panic_ctx())
					} else if let Some(sweet_error) =
						info.payload().downcast_ref::<SweetError>()
					{
						// in wasm the panic location is useless because its nested inside
						// the matcher, use the desc location instead
						#[cfg(target_arch = "wasm32")]
						let bt_str = BacktraceLocation::from_test_desc(desc).file_context();
						#[cfg(not(target_arch = "wasm32"))]
						let bt_str = sweet_error.backtrace_str();
						let payload = sweet_error.message.clone();
						(payload, bt_str)
					} else {
						("Unknown Payload".to_string(), panic_ctx())
					};

				TestResult::Fail(Self::format_backtrace(
					payload,
					bt.unwrap_or_else(|err| err.to_string()),
				))
			}
		}
	}


	pub fn from_test_result(res: Result<(), String>, desc: &TestDesc) -> Self {
		let parsed_result = match (res, &desc.should_panic) {
			(Ok(()), ShouldPanic::No) => Ok(()),
			(Ok(()), ShouldPanic::Yes) => Err("Expected panic".to_string()),
			(Ok(()), ShouldPanic::YesWithMessage(msg)) => {
				Err(format!("Expected panic: {}", msg))
			}
			(Err(err), ShouldPanic::Yes) => {
				Err(format!("Expected panic, received error: {}", err))
			}
			(Err(err), ShouldPanic::YesWithMessage(msg)) => Err(format!(
				"Expected panic '{}', received error: {}",
				msg, err
			)),
			(Err(err), ShouldPanic::No) => Err(err),
		};

		match parsed_result {
			Ok(()) => TestResult::Pass,
			Err(err) => TestResult::Fail(Self::format_backtrace(
				err,
				BacktraceLocation::from_test_desc(desc)
					.file_context()
					.unwrap_or_default(),
			)),
		}
	}

	/// We ignore the catch unwind because we use the panic store
	pub fn flatten_catch_unwind(
		res: Result<Result<(), String>, Box<dyn std::any::Any + Send>>,
	) -> Self {
		match res {
			Ok(Ok(())) => Self::Pass,
			Ok(Err(err)) => Self::Fail(err),
			Err(_) => Self::Pass,
		}
	}

	/// Panics are caught in the hook for backtracing
	/// so we discard the catch_unwind
	pub fn catch_unwind_test_fn(
		func: fn() -> Result<(), String>,
	) -> Option<Self> {
		match std::panic::catch_unwind(func) {
			Ok(Ok(())) => Some(Self::Pass),
			Ok(Err(err)) => Some(Self::Fail(err)),
			Err(_) => None,
		}
	}

	fn format_backtrace(err: String, bt: String) -> String {
		format!("{}\n\n{}", err, bt)
	}
}


impl std::fmt::Display for TestResult {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			TestResult::Pass => write!(f, "Pass"),
			TestResult::Fail(msg) => write!(f, "Fail: {}", msg),
			TestResult::Ignore(Some(msg)) => write!(f, "Ignore: {}", msg),
			TestResult::Ignore(None) => write!(f, "Ignore"),
		}
	}
}
