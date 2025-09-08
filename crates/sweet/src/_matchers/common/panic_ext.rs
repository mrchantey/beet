//! Methods called by [`assert_ext`]
//! Must be called at [`SweetError::BACKTRACE_LEVEL_2`],
//! exactly two levels below the expression that should have its frame
//! captured by the backtrace.
use colorize::AnsiColor;

use crate::prelude::SweetError;
use std::fmt::Debug;
use std::fmt::Display;

/// # Panics
/// always.
/// Must be called at [`SweetError::BACKTRACE_LEVEL_2`]
pub fn panic_with_str(str: impl AsRef<str>) -> ! {
	SweetError::panic(str.as_ref());
}

/// # Panics
/// always.
/// Must be called at [`SweetError::BACKTRACE_LEVEL_2`]
pub fn panic_with_expected_received_display<T1: Display, T2: Display>(
	expected: &T1,
	received: &T2,
) -> ! {
	let expected = format!("{}", expected).green();
	let received = format!("{}", received).red();
	SweetError::panic(format!("Expected: {expected}\nReceived: {received}"));
}
/// # Panics
/// always.
/// Must be called at [`SweetError::BACKTRACE_LEVEL_2`]
pub fn panic_with_expected_received_debug<T1: Debug, T2: Debug>(
	expected: &T1,
	received: &T2,
) -> ! {
	let expected = format!("{:?}", expected).green();
	let received = format!("{:?}", received).red();
	SweetError::panic(format!("Expected: {expected}\nReceived: {received}"));
}
