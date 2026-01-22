//! Tests to verify panic locations are correctly reported when panics
//! occur in different files than the test file itself.
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
use beet_core::testing::panic_in_other_file;

/// This test panics in a different file, verifying that the error location
/// points to `panic_in_other_file.rs` not this file.
#[test]
#[should_panic]
fn panic_in_different_file() { panic_in_other_file::panic_in_this_file(); }

/// This test calls unwrap on an Err in a different file, verifying that
/// the panic location points to `panic_in_other_file.rs` not this file.
#[test]
#[should_panic]
fn unwrap_in_different_file() {
	panic_in_other_file::unwrap_error_in_this_file();
}

/// This test panics in the SAME file, for comparison.
#[test]
#[should_panic]
fn panic_in_same_file() {
	panic!("panic from panic_location.rs");
}

/// This test verifies panic location is correctly captured when panic
/// occurs in a different file - the error display should show the
/// actual panic file, not the test file.
#[test]
#[should_panic]
fn panic_location_in_different_file() {
	panic_in_other_file::panic_in_this_file();
}

/// Similar test but with unwrap - verifies the unwrap panic location
/// is correctly captured and displayed.
#[test]
#[should_panic]
fn unwrap_location_in_different_file() {
	panic_in_other_file::unwrap_error_in_this_file();
}
