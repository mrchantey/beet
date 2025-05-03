use crate::prelude::*;
use colorize::*;
use std::fmt::Debug;

impl<T> Matcher<T> {
	/// Some assertions do not support negation, in that case call this function within the matcher.
	///
	/// This will return an error if the matcher is already negated.
	/// Must be called at [`SweetError::BACKTRACE_LEVEL_2`]
	pub(crate) fn panic_if_negated(&self) {
		if self.negated {
			SweetError::panic("Unsupported: Negation not supported for this matcher, please remove `.not()`".to_string());
		}
	}

	/// # Panics
	/// always.
	/// Must be called at [`SweetError::BACKTRACE_LEVEL_2`]
	pub(crate) fn panic_with_expected_received<T2: Debug, T3: Debug>(
		&self,
		expected: &T2,
		received: &T3,
	) -> ! {
		let mut expected = format!("{:?}", expected)
			.trim_matches('"')
			.to_string()
			.green();

		if self.negated {
			expected = format!("{} {}", "NOT".bold().green(), expected);
		}
		let received = format!("{:?}", received)
			.trim_matches('"')
			.to_string()
			.red();
		SweetError::panic(format!(
			"Expected: {expected}\nReceived: {received}"
		));
	}
}
