//! Glob-based string filtering.
//!
//! This module provides [`GlobFilter`], a type for filtering strings (paths,
//! test names, ..) through an ordered list of include/exclude [rules](GlobRule).
//! It is commonly used for file watching and path matching.
//!
//! Matching is backed by a small vendored glob engine ([`glob_match`]) rather
//! than the `glob` crate, so it needs no filesystem and compiles on no_std.
//! Patterns are matched against the whole string, so `*` and `?` cross `/`
//! (the `glob` crate's `require_literal_separator = false`); `\` in the input is
//! normalized to `/` first so a `/`-pattern matches a `\`-path.
//!
//! # Example
//!
//! ```
//! # use beet_core::prelude::*;
//! let filter = GlobFilter::default()
//!     .with_include("**/*.rs")
//!     .with_exclude("*target*");
//!
//! assert!(filter.passes("src/lib.rs"));
//! assert!(!filter.passes("target/debug/lib.rs"));
//! ```

use crate::prelude::*;

/// A glob-based string filter: an ordered list of rules resolved the way git
/// resolves an ignore file, **last match wins**.
///
/// 1. no rules at all, and everything passes;
/// 2. otherwise the last rule matching the string decides, an include passing
///    it and an exclude rejecting it;
/// 3. and when no rule matches, everything still passes unless the filter holds
///    an include rule anywhere, which makes it an allowlist and rejects the
///    rest.
///
/// Order is what lets a rule be taken back, the thing two unordered lists could
/// never say, since they can only ever resolve as "excludes win":
/// `.with_include("logs/*").with_exclude("logs/*.tmp").with_include("logs/keep.tmp")`
/// is every log but the temporaries, and that one temporary anyway.
///
/// Three authored spellings, each parsed by the same [`LiteralParser`] entry:
/// the allowlist a human writes (`read="guestbook.*"`, `read=["a","b"]`), the
/// two named lists (`{filter:{exclude:["blog/**"]}}`, includes first and
/// excludes last, so the two words read as a human means them), and the rules
/// themselves when the order matters
/// (`{GlobFilter{rules:[Exclude("logs/*"),Include("logs/keep.txt")]}}`).
#[derive(Debug, Default, Clone, PartialEq, Reflect)]
#[reflect(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GlobFilter {
	/// The rules in authored order, resolved last-match-wins by
	/// [`passes`](GlobFilter::passes).
	rules: Vec<GlobRule>,
}

/// One rule of a [`GlobFilter`]: a pattern and what matching it means.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GlobRule {
	/// Strings matching this pattern pass, unless a later rule excludes them.
	Include(GlobPattern),
	/// Strings matching this pattern are rejected, unless a later rule includes
	/// them.
	Exclude(GlobPattern),
}

impl GlobRule {
	/// The pattern this rule matches with, whichever kind it is.
	pub fn pattern(&self) -> &GlobPattern {
		match self {
			Self::Include(pattern) | Self::Exclude(pattern) => pattern,
		}
	}

	/// Whether this rule passes what it matches.
	pub fn is_include(&self) -> bool { matches!(self, Self::Include(_)) }

	/// Whether this rule's pattern matches `text`.
	pub fn matches(&self, text: &str) -> bool { self.pattern().matches(text) }

	/// This rule's kind carrying a different pattern.
	fn with_pattern(&self, pattern: GlobPattern) -> Self {
		match self {
			Self::Include(_) => Self::Include(pattern),
			Self::Exclude(_) => Self::Exclude(pattern),
		}
	}
}

impl core::fmt::Display for GlobRule {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let kind = match self.is_include() {
			true => "include",
			false => "exclude",
		};
		write!(f, "{kind} {}", self.pattern())
	}
}

impl core::fmt::Display for GlobFilter {
	/// The rules in order, comma separated, so what a reader sees is what the
	/// filter resolves.
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		for (index, rule) in self.rules.iter().enumerate() {
			if index > 0 {
				f.write_str(", ")?;
			}
			write!(f, "{rule}")?;
		}
		Ok(())
	}
}

impl GlobFilter {
	/// Parses and validates a glob pattern string.
	///
	/// For use by clap parsers to verify a pattern before inserting.
	pub fn parse_glob_pattern(pattern: &str) -> Result<String, String> {
		validate_glob(pattern)?;
		Ok(pattern.to_string())
	}

	/// Wraps each pattern with wildcards if they don't already have them.
	///
	/// Turns `foo/bar` into `*foo/bar*` which matches any string that contains `foo/bar`.
	pub fn wrap_all_with_wildcard(&mut self) -> &mut Self {
		self.rules = self
			.rules
			.iter()
			.map(|rule| {
				rule.with_pattern(Self::wrap_pattern_with_wildcard(
					rule.pattern().as_str(),
				))
			})
			.collect();
		self
	}

	fn wrap_pattern_with_wildcard(pattern: &str) -> GlobPattern {
		let starts = pattern.starts_with('*');
		let ends = pattern.ends_with('*');
		let wrapped = match (starts, ends) {
			(true, true) => pattern.into(),
			(true, false) => format!("{pattern}*"),
			(false, true) => format!("*{pattern}"),
			(false, false) => format!("*{pattern}*"),
		};
		// Wrapping an already-valid pattern in `*` keeps it valid.
		GlobPattern(wrapped.into())
	}

	/// Extends the rules with an include per item.
	pub fn extend_include<T: AsRef<str>>(
		mut self,
		items: impl IntoIterator<Item = T>,
	) -> Self {
		self.rules.extend(
			items
				.into_iter()
				.map(|item| GlobRule::Include(GlobPattern::new(item.as_ref()))),
		);
		self
	}

	/// Extends the rules with an exclude per item.
	pub fn extend_exclude<T: AsRef<str>>(
		mut self,
		items: impl IntoIterator<Item = T>,
	) -> Self {
		self.rules.extend(
			items
				.into_iter()
				.map(|item| GlobRule::Exclude(GlobPattern::new(item.as_ref()))),
		);
		self
	}

	/// Appends an include rule and returns `&mut Self`.
	pub fn include(&mut self, pattern: &str) -> &mut Self {
		self.rules
			.push(GlobRule::Include(GlobPattern::new(pattern)));
		self
	}

	/// Appends an exclude rule and returns `&mut Self`.
	pub fn exclude(&mut self, pattern: &str) -> &mut Self {
		self.rules
			.push(GlobRule::Exclude(GlobPattern::new(pattern)));
		self
	}

	/// Appends an include rule and returns `Self`.
	pub fn with_include(mut self, pattern: &str) -> Self {
		self.include(pattern);
		self
	}

	/// Appends an exclude rule and returns `Self`.
	pub fn with_exclude(mut self, pattern: &str) -> Self {
		self.exclude(pattern);
		self
	}

	/// Excludes everything inside a directory named `dir` at any depth
	/// (`dir/*` and `*/dir/*`): the churn dirs (`target`, `dist`, `.git`) by
	/// segment, where a bare `*dir*` would also swallow a `targets.md` or a
	/// `distributed/` beside them.
	pub fn with_exclude_dir(self, dir: &str) -> Self {
		self.with_exclude(&format!("{dir}/*"))
			.with_exclude(&format!("*/{dir}/*"))
	}

	/// Returns `true` if there are no rules, ie everything passes.
	pub fn is_empty(&self) -> bool { self.rules.is_empty() }

	/// The rules in order, for a consumer translating this filter into another
	/// matcher's dialect, ie the aws cli's `--include`/`--exclude`.
	pub fn rules(&self) -> &[GlobRule] { &self.rules }

	/// Whether the filter holds any include rule, which is what turns it from a
	/// denylist into an allowlist.
	pub fn has_include(&self) -> bool {
		self.rules.iter().any(GlobRule::is_include)
	}

	/// Checks if a string passes the filter: the last rule to match it decides,
	/// and a string no rule matches passes unless the filter is an allowlist.
	pub fn passes(&self, text: impl AsRef<str>) -> bool {
		let text = text.as_ref();
		self.rules
			.iter()
			.rev()
			.find(|rule| rule.matches(text))
			.map(GlobRule::is_include)
			.unwrap_or_else(|| !self.has_include())
	}

	/// Checks `text` against the include rules ALONE: it passes when there are
	/// none, or when one of them matches.
	///
	/// This half and [`passes_exclude`](Self::passes_exclude) are for a consumer
	/// matching a value that has several names (a component's full type path and
	/// its short path), where the grant is any-of and the denial is all-of.
	/// Everything else wants [`passes`](Self::passes), which resolves the rules
	/// in order.
	pub fn passes_include(&self, text: impl AsRef<str>) -> bool {
		let text = text.as_ref();
		!self.has_include()
			|| self
				.rules
				.iter()
				.any(|rule| rule.is_include() && rule.matches(text))
	}

	/// Checks `text` against the exclude rules ALONE: it passes when none of
	/// them match, see [`passes_include`](Self::passes_include).
	pub fn passes_exclude(&self, text: impl AsRef<str>) -> bool {
		let text = text.as_ref();
		!self
			.rules
			.iter()
			.any(|rule| !rule.is_include() && rule.matches(text))
	}
}

/// A validated glob pattern, stored as a [`SmolStr`] for cheap clones.
///
/// Public because it is the item type of a [`GlobFilter`]'s lists, which a
/// markup struct literal writes into (`{filter:{exclude:["blog/**"]}}`, each
/// string coerced to a validated pattern).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GlobPattern(SmolStr);

impl GlobPattern {
	/// Creates a new [`GlobPattern`], validating it is a valid glob pattern.
	///
	/// # Panics
	///
	/// Panics if the pattern is invalid.
	pub fn new(pattern: &str) -> Self {
		validate_glob(pattern).expect("Invalid glob pattern");
		Self(pattern.into())
	}

	/// Returns the pattern as a string slice.
	pub fn as_str(&self) -> &str { &self.0 }

	/// Checks if the pattern matches the given text.
	///
	/// `\` in `text` is normalized to `/` first, so a `/`-pattern matches a
	/// `\`-path (mirroring the `glob` crate's `matches_path`).
	pub fn matches(&self, text: &str) -> bool {
		if text.contains('\\') {
			glob_match(&self.0, &text.replace('\\', "/"))
		} else {
			glob_match(&self.0, text)
		}
	}
}

impl core::fmt::Display for GlobPattern {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl From<String> for GlobPattern {
	fn from(pattern: String) -> Self { Self::new(&pattern) }
}

impl From<&str> for GlobPattern {
	fn from(pattern: &str) -> Self { Self::new(pattern) }
}

/// Matches `text` against a glob `pattern`.
///
/// Supports `*` (any run, including `/`), `?` (any single char) and `[..]`
/// character classes (with `!` negation and `a-z` ranges). `**` is accepted and
/// behaves as `*` (patterns match the whole string, so `*` already crosses path
/// separators). Brace alternation `{a,b}` is *not* expanded, `{` matches
/// literally, mirroring `glob::Pattern`.
fn glob_match(pattern: &str, text: &str) -> bool {
	let pat: Vec<char> = pattern.chars().collect();
	let txt: Vec<char> = text.chars().collect();
	let mut pi = 0;
	let mut ti = 0;
	// the last `*` seen: (pattern index after the star, text index it absorbs from)
	let mut star: Option<(usize, usize)> = None;
	while ti < txt.len() {
		let advanced = match pat.get(pi) {
			Some('*') => {
				// collapse a run of stars (handles `**`)
				while pat.get(pi) == Some(&'*') {
					pi += 1;
				}
				star = Some((pi, ti));
				true
			}
			Some('?') => {
				pi += 1;
				ti += 1;
				true
			}
			Some('[') => match class_matches(&pat, pi, txt[ti]) {
				Some((true, next)) => {
					pi = next;
					ti += 1;
					true
				}
				// class parsed but the char is not in it
				Some((false, _)) => false,
				// unterminated `[`, treat it as a literal
				None if txt[ti] == '[' => {
					pi += 1;
					ti += 1;
					true
				}
				None => false,
			},
			Some(&pat_char) if pat_char == txt[ti] => {
				pi += 1;
				ti += 1;
				true
			}
			_ => false,
		};
		if advanced {
			continue;
		}
		// mismatch: backtrack to the last `*`, letting it absorb one more char
		match star {
			Some((star_pi, star_ti)) => {
				pi = star_pi;
				ti = star_ti + 1;
				star = Some((star_pi, star_ti + 1));
			}
			None => return false,
		}
	}
	// any remaining pattern must be only stars
	while pat.get(pi) == Some(&'*') {
		pi += 1;
	}
	pi == pat.len()
}

/// Tests `ch` against a `[..]` class starting at `pat[start] == '['`.
///
/// Returns `(matched, index_just_after_the_closing_bracket)`, or [`None`] if the
/// class is unterminated (the caller then treats `[` as a literal).
fn class_matches(
	pat: &[char],
	start: usize,
	ch: char,
) -> Option<(bool, usize)> {
	let mut idx = start + 1;
	let negated = pat.get(idx) == Some(&'!');
	if negated {
		idx += 1;
	}
	let mut matched = false;
	// a `]` is a literal when it is the very first class member
	let mut first = true;
	while idx < pat.len() {
		if pat[idx] == ']' && !first {
			return Some((matched ^ negated, idx + 1));
		}
		first = false;
		// range `a-z`: a `-` flanked by two chars, the `-` not being the closer
		if idx + 2 < pat.len() && pat[idx + 1] == '-' && pat[idx + 2] != ']' {
			if ch >= pat[idx] && ch <= pat[idx + 2] {
				matched = true;
			}
			idx += 3;
		} else {
			if pat[idx] == ch {
				matched = true;
			}
			idx += 1;
		}
	}
	None
}

/// Validates a glob pattern, the only structural requirement being that every
/// `[` character class is terminated by a `]`.
fn validate_glob(pattern: &str) -> Result<(), String> {
	let chars: Vec<char> = pattern.chars().collect();
	let mut idx = 0;
	while idx < chars.len() {
		if chars[idx] != '[' {
			idx += 1;
			continue;
		}
		// scan for the terminator, allowing a leading `!` and a literal first `]`
		let mut end = idx + 1;
		if chars.get(end) == Some(&'!') {
			end += 1;
		}
		let mut first = true;
		let closed = loop {
			match chars.get(end) {
				Some(']') if !first => break true,
				Some(_) => {
					first = false;
					end += 1;
				}
				None => break false,
			}
		};
		if !closed {
			return Err(format!(
				"unterminated character class in glob pattern: {pattern}"
			));
		}
		idx = end + 1;
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn pattern() {
		let pat = GlobPattern::new("*target*");
		pat.matches("foo").xpect_false();
		pat.matches("target").xpect_true();
		pat.matches("foo/target/foo").xpect_true();
	}

	/// `with_exclude_dir` excludes a directory by segment at any depth, and
	/// nothing merely containing its name.
	#[crate::test]
	fn exclude_dir_matches_segments_only() {
		let filter = GlobFilter::default().with_exclude_dir("target");
		filter.passes("target/debug/foo").xpect_false();
		filter.passes("site/target/foo").xpect_false();
		filter.passes("routes/targets.md").xpect_true();
		filter.passes("target").xpect_true();
	}

	#[crate::test]
	fn glob_pattern_new() {
		let gp = GlobPattern::new("*foo*");
		gp.as_str().xpect_eq("*foo*");
		gp.matches("foo").xpect_true();
		gp.matches("bar/foo/baz").xpect_true();
	}

	#[crate::test]
	fn wildcards_and_classes() {
		// `?` matches a single char
		GlobPattern::new("f?o").matches("foo").xpect_true();
		GlobPattern::new("f?o").matches("fooo").xpect_false();
		// character classes, ranges and negation
		GlobPattern::new("[abc]").matches("b").xpect_true();
		GlobPattern::new("[a-c]*").matches("cat").xpect_true();
		GlobPattern::new("[!a-c]*").matches("cat").xpect_false();
		GlobPattern::new("file.[ch]").matches("file.c").xpect_true();
		// `**` behaves as `*`
		GlobPattern::new("**/*.rs").matches("a/b/c.rs").xpect_true();
		// `{}` is literal, not expanded
		GlobPattern::new("{a,b}/*").matches("a/x").xpect_false();
		GlobPattern::new("{a,b}/*").matches("{a,b}/x").xpect_true();
	}

	#[crate::test]
	#[should_panic]
	fn rejects_unterminated_class() { GlobPattern::new("[abc"); }

	#[crate::test]
	fn passes() {
		// test include all but

		let filter = GlobFilter::default().with_exclude("*bar*");
		filter.passes("foo").xpect_true();
		filter.passes("bar").xpect_false();
		filter.passes("foo/bar/bazz").xpect_false();

		// test include only

		let filter = GlobFilter::default()
			.with_include("*foo*")
			.with_exclude("*bar*");

		filter.passes("bing/foo/bong").xpect_true();
		// backslashes are normalized to forward slashes
		filter.passes("bing\\foo\\bong").xpect_true();
		filter.passes("froo").xpect_false();
		filter.passes("bar").xpect_false();

		// test backslashes

		let filter = GlobFilter::default().with_include("*foo/bar*");

		filter.passes_include("foo/bar").xpect_true();
		filter.passes_exclude("foo/bar").xpect_true();

		let filter =
			GlobFilter::default().with_exclude("*apply_style_id_attributes*");
		filter
			.passes_exclude(
				"templating::apply_style_id_attributes::test::nested_template",
			)
			.xpect_false();

		// excludes only
		let filter = GlobFilter::default()
			.with_exclude("*.git*")
			// temp until we get fine grained codegen control
			.with_exclude("*codegen*")
			.with_exclude("*target*");

		filter
			.passes("/home/pete/me/beet/target/snippets/snippets.ron")
			.xpect_false();
		// test multi exclude

		let filter = GlobFilter::default()
			.with_include("**/*.rs")
			.with_exclude("*.git*")
			.with_exclude("*target*");

		filter.passes("/foo/bar/bazz.rs").xpect_true();
		filter.passes("/foo/target/bazz.rs").xpect_false();

		// test or

		let filter = GlobFilter::default()
			.with_include("**/*.rs")
			.with_exclude("{.git,target,html}/**")
			.with_exclude("*codegen*");

		filter.passes("src/lib.rs").xpect_true();
		filter.passes("html/lib.rs").xpect_true();
		filter.passes("src/codegen/mockups.rs").xpect_false();
	}

	/// The rules resolve last-match-wins, so an exclusion can be taken back —
	/// the thing two unordered lists could never say.
	#[crate::test]
	fn last_match_wins() {
		let filter = GlobFilter::default()
			.with_include("logs/*")
			.with_exclude("logs/*.tmp")
			.with_include("logs/keep.tmp");
		filter.passes("logs/run.log").xpect_true();
		filter.passes("logs/scratch.tmp").xpect_false();
		filter.passes("logs/keep.tmp").xpect_true();
		// an include anywhere is still an allowlist, so what no rule matches
		// is rejected
		filter.passes("src/lib.rs").xpect_false();

		// ..and the other order says the opposite about the same two patterns
		GlobFilter::default()
			.with_include("logs/keep.tmp")
			.with_exclude("logs/*.tmp")
			.passes("logs/keep.tmp")
			.xpect_false();
	}

	/// The include/exclude halves ignore order, for a consumer matching a value
	/// that has several names (see [`GlobFilter::passes_include`]).
	#[crate::test]
	fn halves_ignore_order() {
		let filter = GlobFilter::default()
			.with_exclude("logs/*")
			.with_include("logs/keep.txt");
		filter.passes_include("logs/keep.txt").xpect_true();
		filter.passes_exclude("logs/keep.txt").xpect_false();
		filter.passes_include("src/lib.rs").xpect_false();
		filter.passes_exclude("src/lib.rs").xpect_true();
	}

	/// The rules in order, which is what a translated dialect and a log line
	/// both read.
	#[crate::test]
	fn displays_rules_in_order() {
		GlobFilter::default().to_string().xpect_eq("");
		GlobFilter::default()
			.with_exclude("logs/*")
			.with_include("logs/keep.txt")
			.to_string()
			.xpect_eq("exclude logs/*, include logs/keep.txt");
	}

	#[crate::test]
	#[cfg(feature = "json")]
	fn serde_roundtrip() {
		let filter = GlobFilter::default()
			.with_include("**/*.rs")
			.with_exclude("*target*");

		let json = serde_json::to_string(&filter).unwrap();
		let deserialized: GlobFilter = serde_json::from_str(&json).unwrap();

		filter.xpect_eq(deserialized);
		filter.rules().to_vec().xpect_eq(vec![
			GlobRule::Include(GlobPattern::new("**/*.rs")),
			GlobRule::Exclude(GlobPattern::new("*target*")),
		]);
	}

	#[crate::test]
	#[cfg(feature = "json")]
	fn glob_pattern_serde() {
		let pattern = GlobPattern::new("*foo*");
		let json = serde_json::to_string(&pattern).unwrap();
		json.xpect_eq("\"*foo*\"");

		let deserialized: GlobPattern = serde_json::from_str(&json).unwrap();
		deserialized.as_str().xpect_eq("*foo*");
		pattern.xpect_eq(deserialized);
	}
}
