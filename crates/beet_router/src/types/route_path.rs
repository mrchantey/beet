use anyhow::Result;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RoutePath(PathBuf);

impl Default for RoutePath {
	fn default() -> Self { Self(PathBuf::from("/")) }
}

/// routes shouldnt have os specific paths
/// so we allow to_string
impl std::fmt::Display for RoutePath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0.display())
	}
}

impl From<String> for RoutePath {
	fn from(value: String) -> Self { Self(PathBuf::from(value)) }
}
impl From<&str> for RoutePath {
	fn from(value: &str) -> Self { Self(PathBuf::from(value)) }
}

impl Into<PathBuf> for RoutePath {
	fn into(self) -> PathBuf { self.0 }
}

impl std::ops::Deref for RoutePath {
	type Target = PathBuf;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl AsRef<Path> for RoutePath {
	fn as_ref(&self) -> &Path { self.0.as_path() }
}

impl RoutePath {
	pub fn new(path: impl Into<PathBuf>) -> Self { Self(path.into()) }
	/// Creates a route join even if the other route path begins with `/`
	pub fn join(&self, new_path: &RoutePath) -> Self {
		let new_path = new_path.0.strip_prefix("/").unwrap_or(&new_path.0);
		if new_path == Path::new("") {
			self.clone()
		} else {
			Self(self.0.join(&new_path))
		}
	}
	pub fn inner(&self) -> &Path { &self.0 }
	/// given a local path, return a new [`RoutePath`] with:
	/// - any extension removed
	/// - 'index' file stems removed
	/// - leading `/` added if not present
	///
	/// Backslashes are not transformed
	pub fn from_file_path(file_path: impl AsRef<Path>) -> Result<Self> {
		let mut path = file_path.as_ref().to_path_buf();
		path.set_extension("");
		if path
			.file_stem()
			.map(|stem| stem.to_str().map(|stem| stem == "index"))
			.flatten()
			.unwrap_or_default()
		{
			path.pop();
		}
		let mut raw_str = path.to_string_lossy().to_string();
		if raw_str.len() > 1 && raw_str.ends_with('/') {
			raw_str.pop();
		}
		if !raw_str.starts_with('/') {
			raw_str = format!("/{}", raw_str);
		}

		Ok(Self(PathBuf::from(raw_str)))
	}
}

#[derive(Debug, Clone)]
pub struct RoutePathTree {
	/// the name of this level of the tree, ie the directory.
	/// for the root this is called 'root'
	pub name: String,
	/// all paths available at this level of the tree
	pub path: Option<RoutePath>,
	/// All child directories
	pub children: Vec<RoutePathTree>,
}

impl RoutePathTree {
	pub fn flatten(&self) -> Vec<RoutePath> {
		let mut paths = Vec::new();
		fn inner(paths: &mut Vec<RoutePath>, node: &RoutePathTree) {
			if let Some(path) = &node.path {
				paths.push(path.clone());
			}
			for child in node.children.iter() {
				inner(paths, child);
			}
		}
		inner(&mut paths, &self);
		paths
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn route_path() {
		expect(RoutePath::new("hello").to_string()).to_be("hello");

		for (value, expected) in [
			("hello", "/hello"),
			("hello.rs", "/hello"),
			("hello/index.rs", "/hello"),
			("/hello/index/", "/hello"),
			("/hello/index.rs/", "/hello"),
			("/hello/index.rs", "/hello"),
			("/index.rs", "/"),
			("/index.rs/", "/"),
			("/index/hi", "/index/hi"),
			("/index/hi/", "/index/hi"),
		] {
			expect(RoutePath::from_file_path(value).unwrap().to_string())
				.to_be(expected);
		}
	}

	#[test]
	fn join() {
		expect(
			&RoutePath::new("/foo")
				.join(&RoutePath::new("/"))
				.to_string(),
		)
		.to_be("/foo");
	}
}
