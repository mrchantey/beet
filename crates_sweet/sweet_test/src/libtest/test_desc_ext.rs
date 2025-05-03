use test::TestDesc;

pub struct TestDescExt;

impl TestDescExt {
	pub fn is_equal_location(a: &TestDesc, b: &TestDesc) -> bool {
		a.source_file == b.source_file && a.start_line == b.start_line
	}

	/// The `#[test]` macro replaces results with [useless error messages](https://github.com/rust-lang/rust/blob/a25032cf444eeba7652ce5165a2be450430890ba/library/test/src/lib.rs#L234)
	/// so we instead panic and instruct user to use `unwrap`.
	/// Also used by async wasm tests, we dont care what the result is, if ya
	/// want messages, panic! at the disco
	pub fn result_to_panic<T, E>(result: Result<T, E>) {
		match result {
			Ok(_) => {}
			Err(_) => {
				panic!("test returned an Err(). Use `unwrap()` instead to see the contents of the error");
			}
		}
	}

	/// A libtest name is the fully qualified path
	/// ie `test_case::backtrace_error::test::result_builder`
	/// we want to shorten this to just the last part
	pub fn short_name(test: &TestDesc) -> String {
		let path = test.name.to_string();
		path.split("::")
			.last()
			.map(|p| p.to_string())
			.unwrap_or(path)
	}
}
