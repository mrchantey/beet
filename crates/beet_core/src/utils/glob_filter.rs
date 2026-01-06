use bevy::prelude::*;
use clap::Parser;
use std::path::Path;

/// glob for watch patterns
#[derive(Debug, Default, Clone, PartialEq, Reflect, Parser)]
#[reflect(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GlobFilter {
	/// glob for watch patterns, leave empty to include all
	#[arg(long, value_parser = GlobFilter::parse_glob_pattern)]
	include: Vec<GlobPattern>,
	/// glob for ignore patterns
	#[arg(long, value_parser = GlobFilter::parse_glob_pattern)]
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
	/// For use by clap parsers to verify a pattern before inserting
	pub fn parse_glob_pattern(s: &str) -> Result<String, String> {
		// Validate it's a valid pattern
		glob::Pattern::new(s)
			.map_err(|e| format!("Invalid glob pattern: {}", e))?;
		Ok(s.to_string())
	}

	/// Wrap each pattern with wildcards if they dont already have them,
	/// turning `foo/bar` into `*foo/bar*` which matches any path that contains `foo/bar`.
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

	pub fn set_include(mut self, items: Vec<&str>) -> Self {
		self.include = items.iter().map(|w| GlobPattern::new(w)).collect();
		self
	}

	pub fn set_exclude(mut self, items: Vec<&str>) -> Self {
		self.exclude = items.iter().map(|i| GlobPattern::new(i)).collect();
		self
	}
	pub fn extend_include<T: AsRef<str>>(
		mut self,
		items: impl IntoIterator<Item = T>,
	) -> Self {
		self.include
			.extend(items.into_iter().map(|w| GlobPattern::new(w.as_ref())));
		self
	}

	pub fn extend_exclude<T: AsRef<str>>(
		mut self,
		items: impl IntoIterator<Item = T>,
	) -> Self {
		self.exclude
			.extend(items.into_iter().map(|w| GlobPattern::new(w.as_ref())));
		self
	}

	pub fn include(&mut self, pattern: &str) -> &mut Self {
		self.include.push(GlobPattern::new(pattern));
		self
	}

	pub fn exclude(&mut self, pattern: &str) -> &mut Self {
		self.exclude.push(GlobPattern::new(pattern));
		self
	}

	pub fn with_include(mut self, pattern: &str) -> Self {
		self.include.push(GlobPattern::new(pattern));
		self
	}

	pub fn with_exclude(mut self, pattern: &str) -> Self {
		self.exclude.push(GlobPattern::new(pattern));
		self
	}

	pub fn is_empty(&self) -> bool {
		self.include.is_empty() && self.exclude.is_empty()
	}

	/// To pass a path must
	/// 1. not be present in the exclude patterns
	/// 2. be present in the include patterns or the include patterns are empty
	/// Currently converts to string with forward slashes
	pub fn passes(&self, path: impl AsRef<Path>) -> bool {
		self.passes_include(&path) && self.passes_exclude(&path)
	}

	pub fn passes_include(&self, path: impl AsRef<Path>) -> bool {
		self.include.is_empty()
			|| self
				.include
				.iter()
				.any(|watch| watch.matches_path(path.as_ref()))
	}

	pub fn passes_exclude(&self, path: impl AsRef<Path>) -> bool {
		!self
			.exclude
			.iter()
			.any(|watch| watch.matches_path(path.as_ref()))
	}
}


/// A validated glob pattern that stores the pattern as a String
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GlobPattern(String);

impl GlobPattern {
	/// Create a new GlobPattern, validating it's a valid glob pattern
	/// Panics if the pattern is invalid
	pub fn new(pattern: &str) -> Self {
		// Validate it's a valid pattern
		glob::Pattern::new(pattern).expect("Invalid glob pattern");
		Self(pattern.to_string())
	}

	/// Get the pattern as a string slice
	pub fn as_str(&self) -> &str { &self.0 }

	/// Convert to glob::Pattern
	/// Panics if the stored pattern is invalid (should never happen)
	pub fn to_pattern(&self) -> glob::Pattern {
		glob::Pattern::new(&self.0)
			.expect("Invalid glob pattern stored in GlobPattern")
	}

	/// Convert from glob::Pattern
	pub fn from_pattern(pattern: &glob::Pattern) -> Self {
		Self(pattern.as_str().to_string())
	}

	pub fn matches(&self, text: &str) -> bool {
		self.to_pattern().matches(text)
	}

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
	#[cfg(feature = "serde")]
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
	#[cfg(feature = "serde")]
	fn glob_pattern_serde() {
		let pattern = GlobPattern::new("*foo*");
		let json = serde_json::to_string(&pattern).unwrap();
		json.xpect_eq("\"*foo*\"");

		let deserialized: GlobPattern = serde_json::from_str(&json).unwrap();
		deserialized.as_str().xpect_eq("*foo*");
		pattern.xpect_eq(deserialized);
	}
}
