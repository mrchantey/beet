//! Terminal text measurement helpers.

use super::unicode_width;
use beet_core::prelude::*;

/// Count visible columns, skipping ANSI escape sequences.
///
/// Wide (CJK/fullwidth) characters count as 2 columns.
pub fn display_width(s: &str) -> usize {
	let mut w = 0;
	let mut in_esc = false;
	for ch in s.chars() {
		match ch {
			escape::ESC => in_esc = true,
			'm' if in_esc => in_esc = false,
			_ if in_esc => {}
			_ => w += unicode_width(ch) as usize,
		}
	}
	w
}
