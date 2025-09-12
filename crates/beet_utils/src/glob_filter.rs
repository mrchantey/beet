use std::path::Path;

use clap::Parser;
use glob::Pattern;
use glob::PatternError;


/// glob for watch patterns
#[derive(Default, Clone, PartialEq, Parser)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
// #[cfg_attr(feature = "bevy", derive(bevy::prelude::Reflect))]
pub struct GlobFilter {
	/// glob for watch patterns, leave empty to include all
	#[arg(long, value_parser = GlobFilter::parse_glob_pattern)]
	#[cfg_attr(feature = "serde", serde(default, with = "serde_glob_vec",))]
	pub include: Vec<glob::Pattern>,
	/// glob for ignore patterns
	#[arg(long, value_parser = GlobFilter::parse_glob_pattern)]
	#[cfg_attr(feature = "serde", serde(default, with = "serde_glob_vec",))]
	pub exclude: Vec<glob::Pattern>,
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

#[cfg(feature = "serde")]
mod serde_glob_vec {

	pub fn serialize<S>(
		patterns: &Vec<glob::Pattern>,
		serializer: S,
	) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		use serde::ser::SerializeSeq;

		let mut seq = serializer.serialize_seq(Some(patterns.len()))?;
		for pattern in patterns {
			seq.serialize_element(pattern.as_str())?;
		}
		seq.end()
	}

	pub fn deserialize<'de, D>(
		deserializer: D,
	) -> Result<Vec<glob::Pattern>, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::Deserialize;

		let strs = Vec::<String>::deserialize(deserializer)?;
		strs.into_iter()
			.map(|s| glob::Pattern::new(&s).map_err(serde::de::Error::custom))
			.collect()
	}
}


impl GlobFilter {
	pub fn parse_glob_pattern(s: &str) -> Result<glob::Pattern, PatternError> {
		glob::Pattern::new(s)
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

	fn wrap_pattern_with_wildcard(pattern: &str) -> Pattern {
		let starts = pattern.starts_with('*');
		let ends = pattern.ends_with('*');
		Pattern::new(&match (starts, ends) {
			(true, true) => pattern.to_string(),
			(true, false) => format!("{pattern}*"),
			(false, true) => format!("*{pattern}"),
			(false, false) => format!("*{pattern}*"),
		})
		.expect("Failed to create glob pattern")
	}

	pub fn set_include(mut self, watch: Vec<&str>) -> Self {
		self.include = watch
			.iter()
			.map(|w| glob::Pattern::new(w).unwrap())
			.collect();
		self
	}
	pub fn set_exclude(mut self, ignore: Vec<&str>) -> Self {
		self.exclude = ignore
			.iter()
			.map(|w| glob::Pattern::new(w).unwrap())
			.collect();
		self
	}

	pub fn include(&mut self, pattern: &str) -> &mut Self {
		self.include.push(glob::Pattern::new(pattern).unwrap());
		self
	}
	pub fn exclude(&mut self, pattern: &str) -> &mut Self {
		self.exclude.push(glob::Pattern::new(pattern).unwrap());
		self
	}

	pub fn with_include(mut self, pattern: &str) -> Self {
		self.include.push(glob::Pattern::new(pattern).unwrap());
		self
	}
	pub fn with_exclude(mut self, pattern: &str) -> Self {
		self.exclude.push(glob::Pattern::new(pattern).unwrap());
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


impl std::fmt::Debug for GlobFilter {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("GlobFilter")
			.field(
				"include",
				&self
					.include
					.iter()
					.map(|p| p.to_string())
					.collect::<Vec<_>>(),
			)
			.field(
				"exclude",
				&self
					.exclude
					.iter()
					.map(|p| p.to_string())
					.collect::<Vec<_>>(),
			)
			.finish()
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use glob::Pattern;
	#[test]
	fn pattern() {
		let pat = Pattern::new("*target*").unwrap();
		assert!(!pat.matches("foo"));
		assert!(pat.matches("target"));
		assert!(pat.matches("foo/target/foo"));
	}
	#[test]
	fn passes() {
		// test include all but

		let watcher = GlobFilter::default().with_exclude("*bar*");
		assert!(watcher.passes("foo"));
		assert!(!watcher.passes("bar"));
		assert!(!watcher.passes("foo/bar/bazz"));

		// test include only

		let watcher = GlobFilter::default()
			.with_include("*foo*")
			.with_exclude("*bar*");

		assert!(watcher.passes("bing/foo/bong"));
		// backslashes are normalized to forward slashes
		assert!(watcher.passes("bing\\foo\\bong"));
		assert!(!watcher.passes("froo"));
		assert!(!watcher.passes("bar"));

		// test backslashes

		let watcher = GlobFilter::default().with_include("*foo/bar*");

		assert!(watcher.passes_include("foo/bar"));
		assert!(watcher.passes_exclude("foo/bar"));



		let watcher =
			GlobFilter::default().with_exclude("*apply_style_id_attributes*");
		assert!(
			false
				== watcher.passes_exclude(
					"templating::apply_style_id_attributes::test::nested_template"
				)
		);
		// test multi exclude

		let watcher = GlobFilter::default()
			.with_include("**/*.rs")
			.with_exclude("*.git*")
			.with_exclude("*target*");

		assert!(watcher.passes("/foo/bar/bazz.rs"));
		assert!(!watcher.passes("/foo/target/bazz.rs"));

		// test or

		let watcher = GlobFilter::default()
			.with_include("**/*.rs")
			.with_exclude("{.git,target,html}/**")
			.with_exclude("*codegen*");

		assert!(watcher.passes("src/lib.rs"));
		assert!(watcher.passes("html/lib.rs"));
		assert!(!watcher.passes("src/codegen/mockups.rs"));
	}
}
