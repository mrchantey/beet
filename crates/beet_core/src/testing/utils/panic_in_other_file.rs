/// A helper function that panics, used to test that panic locations
/// are correctly reported when they occur in a different file than the test.
pub fn panic_in_this_file() {
	panic!("panic from panic_in_other_file.rs");
}

/// A helper function that returns an error, for testing error location reporting.
pub fn error_in_this_file() -> Result<(), String> {
	Err("error from panic_in_other_file.rs".into())
}

/// A helper function that calls unwrap on an Err, causing a panic with location info.
pub fn unwrap_error_in_this_file() {
	let result: Result<(), &str> =
		Err("unwrap error from panic_in_other_file.rs");
	result.unwrap();
}
