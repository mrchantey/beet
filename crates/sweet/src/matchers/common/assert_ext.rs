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
	expected: T1,
	received: T2,
) -> ! {
	panic_ext::panic_expected_received_display(expected, received)
}
/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
pub fn panic_expected_received_debug<T1: Debug, T2: Debug>(
	expected: T1,
	received: T2,
) -> ! {
	panic_ext::panic_expected_received_debug(expected, received)
}
/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
pub fn panic_expected_received_debug_display<T1: Debug, T2: Display>(
	expected: T1,
	received: T2,
) -> ! {
	panic_ext::panic_expected_received_debug_display(expected, received)
}
/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
pub fn panic_expected_received_display_debug<T1: Display, T2: Debug>(
	expected: T1,
	received: T2,
) -> ! {
	panic_ext::panic_expected_received_display_debug(expected, received)
}

/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
pub fn assert_expected_received_display<Expected: Display, Received: Display>(
	expected: Expected,
	received: impl IntoMaybeNotDisplay<Received>,
) where
	Received: PartialEq<Expected>,
{
	let received = received.into_maybe_not();
	if let Err(expected) = received.compare_display(&expected) {
		panic_ext::panic_expected_received_display(&expected, received.inner());
	}
}

/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
pub fn assert_expected_received_debug<T1: Debug, T2: PartialEq<T1> + Debug>(
	expected: T1,
	received: impl IntoMaybeNotDisplay<T2>,
) {
	let received = received.into_maybe_not();
	if let Err(expected) = received.compare_debug(&expected) {
		panic_ext::panic_expected_received_display_debug(&expected, &received);
	}
}
/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
pub fn assert_result_expected_received_display<
	Expected: Display,
	Received: Display,
>(
	result: bool,
	expected: Expected,
	received: MaybeNot<Received>,
) -> MaybeNot<Received> {
	if let Err(expected) = received.passes_display(result, &expected) {
		panic_ext::panic_expected_received_display(&expected, received.inner());
	}
	received
}


/// Panics if the values are not equal
pub fn assert_diff<Received: AsRef<str>>(
	expected: impl AsRef<str>,
	received: MaybeNot<Received>,
) -> MaybeNot<Received> {
	let expected = expected.as_ref();
	let received_str = received.inner().as_ref();
	let is_match = expected == received_str;
	match (is_match, received.is_negated()) {
		(true, false) => received,
		(false, true) => received,
		(true, true) => {
			panic_ext::panic_expected_received_display(
				"NOT to be string",
				received_str,
			);
		}
		(false, false) => {
			panic_ext::panic_str(&crate::utils::pretty_diff::inline_diff(
				expected,
				received_str,
			));
		}
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
