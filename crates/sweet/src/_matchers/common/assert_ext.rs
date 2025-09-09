//! These are assertions designed to be called directly from matchers,
//! Must be called at [`SweetError::BACKTRACE_LEVEL_3`],
//! exactly two levels below the expression that should have its frame
//! captured by the backtrace.
use std::fmt::Debug;
use std::fmt::Display;

use crate::prelude::*;


/// Panics if the result is false
pub fn assert(result: bool, msg: &str) {
	if !result {
		panic_ext::panic_str(msg);
	}
}



/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
pub fn panic_expected_received_display<T1: Display, T2: Display>(
	expected: &T1,
	received: &T2,
) {
	panic_ext::panic_expected_received_display(expected, received)
}
/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
pub fn panic_expected_received_debug<T1: Debug, T2: Debug>(
	expected: &T1,
	received: &T2,
) {
	panic_ext::panic_expected_received_debug(expected, received)
}
/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
pub fn panic_expected_received_debug_display<T1: Debug, T2: Display>(
	expected: &T1,
	received: &T2,
) {
	panic_ext::panic_expected_received_debug_display(expected, received)
}
/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
pub fn panic_expected_received_display_debug<T1: Display, T2: Debug>(
	expected: &T1,
	received: &T2,
) {
	panic_ext::panic_expected_received_display_debug(expected, received)
}

/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
pub fn assert_expected_received_display<
	T1: PartialEq<T2> + Display,
	T2: Display,
>(
	expected: &T1,
	received: &T2,
) {
	if expected != received {
		panic_ext::panic_expected_received_display(expected, received)
	}
}

/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
pub fn assert_expected_received_debug<T1: PartialEq<T2> + Debug, T2: Debug>(
	expected: &T1,
	received: &T2,
) {
	if expected != received {
		panic_ext::panic_expected_received_debug(expected, received)
	}
}


/// Panics if the values are not equal
pub fn assert_diff(expected: impl AsRef<str>, received: impl AsRef<str>) {
	let expected = expected.as_ref();
	let received = received.as_ref();
	if expected != received {
		panic_ext::panic_str(&crate::utils::pretty_diff::inline_diff(
			expected, received,
		));
	}
}


/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
pub fn assert_some_with_received_display<T>(received: Option<T>)
where
	Option<T>: Display,
{
	if received.is_none() {
		panic_ext::panic_expected_received_display(&"Some", &received);
	}
}

/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
pub fn assert_none_with_received_display<T>(received: Option<T>)
where
	Option<T>: Display,
{
	if received.is_some() {
		panic_ext::panic_expected_received_display(&"None", &received);
	}
}
