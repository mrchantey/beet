//! String helpers with no natural home on a beet type.
use alloc::vec::Vec;

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

/// The Levenshtein distance between `a` and `b`: the fewest single-char
/// insertions, deletions and substitutions turning one into the other.
///
/// ```rust
/// # use beet_core::prelude::*;
/// str_ext::edit_distance("elevate", "elevated").xpect_eq(1);
/// str_ext::edit_distance("stage", "stag").xpect_eq(1);
/// str_ext::edit_distance("", "abc").xpect_eq(3);
/// ```
pub fn edit_distance(a: &str, b: &str) -> usize {
	let b = b.chars().collect::<Vec<_>>();
	// one row of the dynamic programming table, rolled forward per char of `a`
	let mut row = (0..=b.len()).collect::<Vec<_>>();
	for (i, a_char) in a.chars().enumerate() {
		let mut diagonal = row[0];
		row[0] = i + 1;
		for (j, b_char) in b.iter().enumerate() {
			let substitution = diagonal + usize::from(a_char != *b_char);
			diagonal = row[j + 1];
			row[j + 1] = substitution.min(row[j] + 1).min(diagonal + 1);
		}
	}
	row[b.len()]
}

/// The candidate `target` most plausibly misspells, for a "did you mean"
/// hint: the nearest by [`edit_distance`] within a third of the longer
/// length (at least two edits), the first listed on a tie. `None` when
/// nothing is that close.
///
/// ```rust
/// # use beet_core::prelude::*;
/// str_ext::closest("elevate", ["roll", "elevated", "dry-run"])
/// 	.xpect_eq(Some("elevated"));
/// str_ext::closest("verbose", ["roll", "elevated"]).xpect_none();
/// ```
pub fn closest<'a>(
	target: &str,
	candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
	candidates
		.into_iter()
		.map(|candidate| (edit_distance(target, candidate), candidate))
		.filter(|(distance, candidate)| {
			*distance <= (target.len().max(candidate.len()) / 3).max(2)
		})
		.min_by_key(|(distance, _)| *distance)
		.map(|(_, candidate)| candidate)
}
