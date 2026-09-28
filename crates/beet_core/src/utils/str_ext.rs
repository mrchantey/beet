//! String helpers with no natural home on a beet type.

/// The trimmed, non-empty items of a comma separated list, the shape a
/// list-valued flag takes (`--only=a,b`, `--recipients=age1..,age1..`).
///
/// Trimming is the part a caller has to want: a flag whose items may carry
/// significant leading space parses its own list.
///
/// ```rust
/// # use beet_core::prelude::*;
/// str_ext::csv(" a, ,b ").collect::<Vec<_>>().xpect_eq(vec!["a", "b"]);
/// ```
pub fn csv(list: &str) -> impl Iterator<Item = &str> {
	list.split(',')
		.map(str::trim)
		.filter(|item| !item.is_empty())
}
