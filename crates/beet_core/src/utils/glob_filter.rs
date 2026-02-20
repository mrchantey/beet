//! Glob-based path filtering.
//!
//! This module provides [`GlobFilter`], a type for filtering paths using
//! include and exclude glob patterns. It's commonly used for file watching
//! and path matching operations.
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

use bevy::prelude::*;
use std::path::Path;

/// A glob-based path filter with include and exclude patterns.
///
/// To pass a path must:
/// 1. Not match any exclude patterns
/// 2. Match at least one include pattern (or include patterns are empty)
#[derive(Debug, Default, Clone, PartialEq, Reflect)]
#[reflect(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GlobFilter {
	/// Glob patterns for paths to include. Leave empty to include all.
	include: Vec<GlobPattern>,
	/// Glob patterns for paths to exclude.
	exclude: Vec<GlobPattern>,
}

impl std::fmt::Display for GlobFilter {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let include = self
			.include
			.iter()
			.map(|p| p.as_str())
			.collect::<Vec<_>>()
			.join(", ");
		let exclude = self
			.exclude
			.iter()
			.map(|p| p.as_str())
			.collect::<Vec<_>>()
			.join(", ");
		write!(f, "include: {}\nexclude: {}", include, exclude)
	}
}

impl GlobFilter {
	/// Parses and validates a glob pattern string.
	///
	/// For use by clap parsers to verify a pattern before inserting.
	pub fn parse_glob_pattern(s: &str) -> Result<String, String> {
		// Validate it's a valid pattern
		glob::Pattern::new(s)
			.map_err(|e| format!("Invalid glob pattern: {}", e))?;
		Ok(s.to_string())
	}

	/// Wraps each pattern with wildcards if they don't already have them.
	///
	/// Turns `foo/bar` into `*foo/bar*` which matches any path that contains `foo/bar`.
	pub fn wrap_all_with_wildcard(&mut self) -> &mut Self {
		self.include = self
			.include
			.iter()
			.map(|p| Self::wrap_pattern_with_wildcard(p.as_str()))
			.collect();
		self.exclude = self
			.exclude
			.iter()
			.map(|p| Self::wrap_pattern_with_wildcard(p.as_str()))
			.collect();
		self
	}

	fn wrap_pattern_with_wildcard(pattern: &str) -> GlobPattern {
		let starts = pattern.starts_with('*');
		let ends = pattern.ends_with('*');
		let wrapped = match (starts, ends) {
			(true, true) => pattern.to_string(),
			(true, false) => format!("{pattern}*"),
			(false, true) => format!("*{pattern}"),
			(false, false) => format!("*{pattern}*"),
		};
		GlobPattern(wrapped)
	}

	/// Sets the include patterns, replacing any existing ones.
	pub fn set_include(mut self, items: Vec<&str>) -> Self {
		self.include = items.iter().map(|w| GlobPattern::new(w)).collect();
		self
	}

	/// Sets the exclude patterns, replacing any existing ones.
	pub fn set_exclude(mut self, items: Vec<&str>) -> Self {
		self.exclude = items.iter().map(|i| GlobPattern::new(i)).collect();
		self
	}

	/// Extends the include patterns with additional items.
	pub fn extend_include<T: AsRef<str>>(
		mut self,
		items: impl IntoIterator<Item = T>,
	) -> Self {
		self.include
			.extend(items.into_iter().map(|w| GlobPattern::new(w.as_ref())));
		self
	}

	/// Extends the exclude patterns with additional items.
	pub fn extend_exclude<T: AsRef<str>>(
		mut self,
		items: impl IntoIterator<Item = T>,
	) -> Self {
		self.exclude
			.extend(items.into_iter().map(|w| GlobPattern::new(w.as_ref())));
		self
	}

	/// Adds an include pattern and returns `&mut Self`.
	pub fn include(&mut self, pattern: &str) -> &mut Self {
		self.include.push(GlobPattern::new(pattern));
		self
	}

	/// Adds an exclude pattern and returns `&mut Self`.
	pub fn exclude(&mut self, pattern: &str) -> &mut Self {
		self.exclude.push(GlobPattern::new(pattern));
		self
	}

	/// Adds an include pattern and returns `Self`.
	pub fn with_include(mut self, pattern: &str) -> Self {
		self.include.push(GlobPattern::new(pattern));
		self
	}

	/// Adds an exclude pattern and returns `Self`.
	pub fn with_exclude(mut self, pattern: &str) -> Self {
		self.exclude.push(GlobPattern::new(pattern));
		self
	}

	/// Returns `true` if there are no include or exclude patterns.
	pub fn is_empty(&self) -> bool {
		self.include.is_empty() && self.exclude.is_empty()
	}

	/// Checks if a path passes the filter.
	///
	/// To pass a path must:
	/// 1. Not be present in the exclude patterns
	/// 2. Be present in the include patterns or the include patterns are empty
	///
	/// Currently converts paths to strings with forward slashes.
	pub fn passes(&self, path: impl AsRef<Path>) -> bool {
		self.passes_include(&path) && self.passes_exclude(&path)
	}

	/// Checks if a path passes the include filter.
	pub fn passes_include(&self, path: impl AsRef<Path>) -> bool {
		self.include.is_empty()
			|| self
				.include
				.iter()
				.any(|watch| watch.matches_path(path.as_ref()))
	}

	/// Checks if a path passes the exclude filter.
	pub fn passes_exclude(&self, path: impl AsRef<Path>) -> bool {
		!self
			.exclude
			.iter()
			.any(|watch| watch.matches_path(path.as_ref()))
	}
}


/// A validated glob pattern that stores the pattern as a String.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GlobPattern(String);

impl GlobPattern {
	/// Creates a new [`GlobPattern`], validating it's a valid glob pattern.
	///
	/// # Panics
	///
	/// Panics if the pattern is invalid.
	pub fn new(pattern: &str) -> Self {
		// Validate it's a valid pattern
		glob::Pattern::new(pattern).expect("Invalid glob pattern");
		Self(pattern.to_string())
	}

	/// Returns the pattern as a string slice.
	pub fn as_str(&self) -> &str { &self.0 }

	/// Converts to a [`glob::Pattern`].
	///
	/// # Panics
	///
	/// Panics if the stored pattern is invalid (should never happen).
	pub fn to_pattern(&self) -> glob::Pattern {
		glob::Pattern::new(&self.0)
			.expect("Invalid glob pattern stored in GlobPattern")
	}

	/// Creates a [`GlobPattern`] from a [`glob::Pattern`].
	pub fn from_pattern(pattern: &glob::Pattern) -> Self {
		Self(pattern.as_str().to_string())
	}

	/// Checks if the pattern matches the given text.
	pub fn matches(&self, text: &str) -> bool {
		self.to_pattern().matches(text)
	}

	/// Checks if the pattern matches the given path.
	pub fn matches_path(&self, path: impl AsRef<Path>) -> bool {
		self.to_pattern().matches_path(path.as_ref())
	}
}

impl std::fmt::Display for GlobPattern {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl From<String> for GlobPattern {
	fn from(s: String) -> Self {
		glob::Pattern::new(&s).expect("Invalid glob pattern");
		Self(s)
	}
}

impl From<&str> for GlobPattern {
	fn from(s: &str) -> Self { Self::new(s) }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use glob::Pattern;

	#[test]
	fn pattern() {
		let pat = Pattern::new("*target*").unwrap();
		pat.matches("foo").xpect_false();
		pat.matches("target").xpect_true();
		pat.matches("foo/target/foo").xpect_true();
	}

	#[test]
	fn glob_pattern_new() {
		let gp = GlobPattern::new("*foo*");
		gp.as_str().xpect_eq("*foo*");
		gp.to_pattern().matches("foo").xpect_true();
		gp.to_pattern().matches("bar/foo/baz").xpect_true();
	}

	#[test]
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

	#[test]
	#[cfg(feature = "json")]
	fn serde_roundtrip() {
		let filter = GlobFilter::default()
			.with_include("**/*.rs")
			.with_exclude("*target*");

		let json = serde_json::to_string(&filter).unwrap();
		let deserialized: GlobFilter = serde_json::from_str(&json).unwrap();

		filter.xpect_eq(deserialized);
		filter.include.len().xpect_eq(1);
		filter.exclude.len().xpect_eq(1);
		filter.include[0].as_str().xpect_eq("**/*.rs");
		filter.exclude[0].as_str().xpect_eq("*target*");
	}

	#[test]
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
