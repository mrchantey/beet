use beet_core::prelude::*;
use test::ShouldPanic;
use test::TestDesc;
use test::TestDescAndFn;
use test::TestType;

/// Extension trait for building and modifying TestDesc instances with a fluent API.
///
/// # Examples
///
/// ```
/// use sweet::prelude::*;
///
/// // Create a test descriptor with should_panic
/// let desc = test_ext::new_desc("my_test", file!())
///     .with_should_panic();
///
/// // Chain multiple builders
/// let desc = test_ext::new_desc("my_test", file!())
///     .with_ignore(true)
///     .with_should_panic_message("expected error");
///
/// // Use with TestDescAndFn
/// let test = test_ext::new("my_test", file!(), || Ok(()))
///     .with_should_panic()
///     .with_ignore(false);
/// ```
pub trait TestDescExt {
	/// Get reference to the test descriptor
	fn desc(&self) -> &TestDesc;
	/// Get mutable reference to the test descriptor
	fn desc_mut(&mut self) -> &mut TestDesc;

	/// Get the short name of the test (without module path)
	fn short_name(&self) -> &str {
		let full_name = self.desc().name.as_slice();
		match full_name.rfind("::") {
			Some(idx) => &full_name[idx + 2..],
			None => full_name,
		}
	}

	fn path(&self) -> WsPathBuf { WsPathBuf::new(self.desc().source_file) }

	fn start(&self) -> LineCol {
		LineCol {
			line: self.desc().start_line as u32,
			col: self.desc().start_col as u32,
		}
	}

	fn end(&self) -> LineCol {
		LineCol {
			line: self.desc().end_line as u32,
			col: self.desc().end_col as u32,
		}
	}

	/// Spits the file name at 'src' and returns the final part
	fn short_file(&self) -> &str {
		let full_file = self.desc().source_file;
		match full_file.rfind("src/") {
			Some(idx) => &full_file[idx + 4..],
			None => full_file,
		}
	}

	fn short_file_and_name(&self) -> String {
		format!("{} · {}", self.short_file(), self.short_name())
	}

	/// Set whether the test should be ignored.
	fn with_ignore(mut self, should_ignore: bool) -> Self
	where
		Self: Sized,
	{
		self.desc_mut().ignore = should_ignore;
		self
	}

	/// Set the ignore message. Also sets `ignore` to `true`.
	fn with_ignore_message(mut self, message: &'static str) -> Self
	where
		Self: Sized,
	{
		self.desc_mut().ignore = true;
		self.desc_mut().ignore_message = Some(message);
		self
	}

	/// Set that the test should panic.
	fn with_should_panic(mut self) -> Self
	where
		Self: Sized,
	{
		self.desc_mut().should_panic = ShouldPanic::Yes;
		self
	}

	/// Set that the test should panic with a specific message.
	fn with_should_panic_message(mut self, message: &'static str) -> Self
	where
		Self: Sized,
	{
		self.desc_mut().should_panic = ShouldPanic::YesWithMessage(message);
		self
	}

	/// Set the test type.
	fn with_test_type(mut self, test_type: TestType) -> Self
	where
		Self: Sized,
	{
		self.desc_mut().test_type = test_type;
		self
	}

	/// Set whether the test should compile fail.
	fn with_compile_fail(mut self, should_fail: bool) -> Self
	where
		Self: Sized,
	{
		self.desc_mut().compile_fail = should_fail;
		self
	}

	/// Set whether the test should not run.
	fn with_no_run(mut self, should_not_run: bool) -> Self
	where
		Self: Sized,
	{
		self.desc_mut().no_run = should_not_run;
		self
	}

	/// Set the source location information.
	fn with_source_location(
		mut self,
		file: &'static str,
		start_line: usize,
		start_col: usize,
		end_line: usize,
		end_col: usize,
	) -> Self
	where
		Self: Sized,
	{
		let desc = self.desc_mut();
		desc.source_file = file;
		desc.start_line = start_line;
		desc.start_col = start_col;
		desc.end_line = end_line;
		desc.end_col = end_col;
		self
	}
}

impl TestDescExt for TestDesc {
	fn desc(&self) -> &TestDesc { self }
	fn desc_mut(&mut self) -> &mut TestDesc { self }
}

impl TestDescExt for TestDescAndFn {
	fn desc(&self) -> &TestDesc { &self.desc }
	fn desc_mut(&mut self) -> &mut TestDesc { &mut self.desc }
}
