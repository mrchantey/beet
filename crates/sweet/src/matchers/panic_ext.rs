//! Panic and assertion helpers for matchers.
//! All functions use `#[track_caller]` to capture the correct source location.
use crate::prelude::*;
use std::fmt::Debug;
use std::fmt::Display;

// ============================================================================
// Panic functions - always panic with formatted messages
// ============================================================================

/// Panics with a string message.
#[track_caller]
pub fn panic_str(msg: impl AsRef<str>) -> ! {
	std::panic::panic_any(msg.as_ref().to_string());
}

/// Panics with formatted expected/received using Display for both.
#[track_caller]
pub fn panic_expected_received_display<T1: Display, T2: Display>(
	expected: T1,
	received: T2,
) -> ! {
	let expected = paint_ext::green(expected);
	let received = paint_ext::red(received);
	std::panic::panic_any(format!(
		"Expected: {expected}\nReceived: {received}"
	));
}

/// Panics with formatted expected/received using Debug for both.
#[track_caller]
pub fn panic_expected_received_debug<T1: Debug, T2: Debug>(
	expected: T1,
	received: T2,
) -> ! {
	let expected = paint_ext::green(format!("{:?}", expected));
	let received = paint_ext::red(format!("{:?}", received));
	std::panic::panic_any(format!(
		"Expected: {expected}\nReceived: {received}"
	));
}

/// Panics with formatted expected/received using Debug for expected, Display for received.
#[track_caller]
pub fn panic_expected_received_debug_display<T1: Debug, T2: Display>(
	expected: T1,
	received: T2,
) -> ! {
	let expected = paint_ext::green(format!("{:?}", expected));
	let received = paint_ext::red(received);
	std::panic::panic_any(format!(
		"Expected: {expected}\nReceived: {received}"
	));
}

/// Panics with formatted expected/received using Display for expected, Debug for received.
#[track_caller]
pub fn panic_expected_received_display_debug<T1: Display, T2: Debug>(
	expected: T1,
	received: T2,
) -> ! {
	let expected = paint_ext::green(expected);
	let received = paint_ext::red(format!("{:?}", received));
	std::panic::panic_any(format!(
		"Expected: {expected}\nReceived: {received}"
	));
}

// ============================================================================
// Assert functions - check conditions then panic if failed
// ============================================================================

/// Panics if the result is false.
#[track_caller]
pub fn assert(result: bool, msg: impl AsRef<str>) {
	if !result {
		panic_str(msg);
	}
}

/// Asserts equality using Display formatting.
#[track_caller]
pub fn assert_expected_received_display<Expected: Display, Received: Display>(
	expected: Expected,
	received: impl IntoMaybeNotDisplay<Received>,
) where
	Received: PartialEq<Expected>,
{
	let received = received.into_maybe_not();
	if let Err(expected) = received.compare_display(&expected) {
		panic_expected_received_display(&expected, received.inner());
	}
}

/// Asserts equality using Debug formatting.
#[track_caller]
pub fn assert_expected_received_debug<T1: Debug, T2: PartialEq<T1> + Debug>(
	expected: T1,
	received: impl IntoMaybeNotDisplay<T2>,
) {
	let received = received.into_maybe_not();
	if let Err(expected) = received.compare_debug(&expected) {
		panic_expected_received_display_debug(&expected, &received);
	}
}

/// Asserts a boolean result with expected/received Display formatting.
#[track_caller]
pub fn assert_result_expected_received_display<
	Expected: Display,
	Received: Display,
>(
	result: bool,
	expected: Expected,
	received: MaybeNot<Received>,
) -> MaybeNot<Received> {
	if let Err(expected) = received.passes_display(result, &expected) {
		panic_expected_received_display(&expected, received.inner());
	}
	received
}

/// Asserts string equality with diff output on failure.
#[track_caller]
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
			panic_expected_received_display("NOT to be string", received_str);
		}
		(false, false) => {
			panic_str(&pretty_diff::inline_diff(expected, received_str));
		}
	}
}

/// Asserts that an Option is Some.
#[track_caller]
pub fn assert_some_with_received_display<T>(received: Option<T>)
where
	Option<T>: Display,
{
	if received.is_none() {
		panic_expected_received_display(&"Some", &received);
	}
}

/// Asserts that an Option is None.
#[track_caller]
pub fn assert_none_with_received_display<T>(received: Option<T>)
where
	Option<T>: Display,
{
	if received.is_some() {
		panic_expected_received_display(&"None", &received);
	}
}
