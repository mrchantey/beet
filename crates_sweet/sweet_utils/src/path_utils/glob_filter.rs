use clap::Parser;
use glob::PatternError;
use std::path::Path;


/// glob for watch patterns
#[derive(Default, Clone, Parser)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GlobFilter {
	/// glob for watch patterns, leave empty to include all
	#[arg(long, value_parser = parse_glob_pattern)]
	#[cfg_attr(
		feature = "serde",
		serde(
			serialize_with = "serialize_patterns",
			deserialize_with = "deserialize_patterns"
		)
	)]
	pub include: Vec<glob::Pattern>,
	/// glob for ignore patterns
	#[arg(long, value_parser = parse_glob_pattern)]
	#[cfg_attr(
		feature = "serde",
		serde(
			serialize_with = "serialize_patterns",
			deserialize_with = "deserialize_patterns"
		)
	)]
	pub exclude: Vec<glob::Pattern>,
}

fn parse_glob_pattern(s: &str) -> Result<glob::Pattern, PatternError> {
	glob::Pattern::new(s)
}

#[cfg(feature = "serde")]
fn serialize_patterns<S>(
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

#[cfg(feature = "serde")]
fn deserialize_patterns<'de, D>(
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


impl GlobFilter {
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

	pub fn with_include(mut self, watch: &str) -> Self {
		self.include.push(glob::Pattern::new(watch).unwrap());
		self
	}
	pub fn with_exclude(mut self, watch: &str) -> Self {
		self.exclude.push(glob::Pattern::new(watch).unwrap());
		self
	}
	/// Currently converts to string with forward slashes
	pub fn passes(&self, path: impl AsRef<Path>) -> bool {
		// TODO this is presumptuous
		let path_str = path.as_ref().to_string_lossy().replace('\\', "/");
		// let path = Path::new(&path_str);
		let pass_include =
			self.include.iter().any(|watch| watch.matches(&path_str))
				|| self.include.is_empty();
		let pass_exclude = self
			.exclude
			.iter()
			.all(|watch| watch.matches(&path_str) == false);
		pass_include && pass_exclude
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
		// let mut watcher = GlobFilter::default();
		// expect(watcher.exclude
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

		let watcher = GlobFilter::default().with_include("foo/bar");

		assert!(watcher.passes("foo/bar"));
		// backslashes are normalized to forward slashes
		assert!(watcher.passes("foo\\bar"));

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
