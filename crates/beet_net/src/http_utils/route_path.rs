use crate::prelude::*;
use beet_core::prelude::*;
use http::Uri;
use http::uri::InvalidUri;
use std::path::Path;
use std::path::PathBuf;


/// Describes an absolute path to a route, beginning with `/`.
///
/// ```txt
/// https://example.com/foo/bar.txt
///                    ^^^^^^^^^^^^
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct RoutePath(pub PathBuf);

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

/// The uri path of the request, if the request has no leading slash
/// this will be empty.
impl FromRequestMeta<Self> for RoutePath {
	fn from_request_meta(req: &RequestMeta) -> Result<Self, Response> {
		let path = req.uri.path();
		Self::new(path).xok()
	}
}

impl From<String> for RoutePath {
	fn from(value: String) -> Self { Self::new(value) }
}
impl From<&str> for RoutePath {
	fn from(value: &str) -> Self { Self::new(value) }
}

impl From<PathBuf> for RoutePath {
	fn from(value: PathBuf) -> Self { Self::new(value) }
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
impl AsRef<str> for RoutePath {
	fn as_ref(&self) -> &str { self.0.to_str().unwrap_or_default() }
}

impl Into<Request> for RoutePath {
	fn into(self) -> Request { Request::new(HttpMethod::Get, self) }
}

impl TryInto<Uri> for RoutePath {
	type Error = InvalidUri;

	fn try_into(self) -> Result<Uri, Self::Error> {
		Uri::try_from(self.0.to_string_lossy().as_ref())
	}
}

impl RoutePath {
	/// Creates a new [`RoutePath`] from a string, ensuring it starts with `/`
	pub fn new(path: impl Into<PathBuf>) -> Self {
		let path_buf = path.into();
		let path_str = path_buf.to_string_lossy();
		if path_str.starts_with('/') {
			Self(path_buf)
		} else {
			Self(PathBuf::from(format!("/{}", path_str)))
		}
	}


	/// when joining with other paths ensure that the path
	/// does not start with a leading slash, as this would
	/// cause the path to be treated as an absolute path
	pub fn as_relative(&self) -> &Path {
		self.0.strip_prefix("/").unwrap_or(&self.0)
	}

	/// Creates a route join even if the other route path begins with `/`
	pub fn join(&self, new_path: impl AsRef<Path>) -> Self {
		let new_path = new_path.as_ref();
		let new_path = new_path.strip_prefix("/").unwrap_or(new_path);
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


/// Represents all route paths in an application, structured as a tree.
#[derive(Debug, Clone, Resource)]
pub struct RoutePathTree {
	/// The full route path for this node
	pub route: RoutePath,
	/// All entities with an [`Endpoint`] that matches this path
	pub endpoints: Vec<Entity>,
	/// All child directories
	pub children: Vec<RoutePathTree>,
}

impl RoutePathTree {
	pub fn name(&self) -> &str {
		self.route
			.0
			.file_name()
			.and_then(|name| name.to_str())
			.unwrap_or("")
	}

	/// Returns true if this node has endpoints
	pub fn contains_endpoints(&self) -> bool { !self.endpoints.is_empty() }

	/// Builds a RoutePathTree from a list of (Entity, RoutePath)
	pub fn from_paths(paths: Vec<(Entity, RoutePath)>) -> Self {
		use std::collections::HashMap;
		// Helper to split a RoutePath into segments
		fn split_segments(path: &RoutePath) -> Vec<String> {
			let s = path.0.to_string_lossy();
			s.split('/')
				.filter(|seg| !seg.is_empty())
				.map(|seg| seg.to_string())
				.collect()
		}

		// Build a tree-like structure
		#[derive(Default)]
		struct Node {
			children: HashMap<String, Node>,
			entities: Vec<Entity>,
		}

		let mut root = Node::default();
		for (ent, route_path) in &paths {
			let segments = split_segments(route_path);
			let mut node = &mut root;
			for (i, seg) in segments.iter().enumerate() {
				node = node.children.entry(seg.clone()).or_default();
				if i == segments.len() - 1 {
					node.entities.push(*ent);
				}
			}
			// Handle root path
			if segments.is_empty() {
				node.entities.push(*ent);
			}
		}

		// Recursively build RoutePathTree from Node
		fn build_tree(route: RoutePath, node: &Node) -> RoutePathTree {
			let mut children: Vec<RoutePathTree> = node
				.children
				.iter()
				.map(|(child_name, child_node)| {
					let child_route = route.join(&RoutePath::new(child_name));
					build_tree(child_route, child_node)
				})
				.collect();
			children
				.sort_by(|a, b| a.route.to_string().cmp(&b.route.to_string()));
			RoutePathTree {
				route,
				endpoints: node.entities.clone(),
				children,
			}
		}

		build_tree(RoutePath::new("/"), &root)
	}

	pub fn flatten(&self) -> Vec<RoutePath> {
		let mut paths = Vec::new();
		fn inner(paths: &mut Vec<RoutePath>, node: &RoutePathTree) {
			if node.contains_endpoints() {
				paths.push(node.route.clone());
			}
			for child in node.children.iter() {
				inner(paths, child);
			}
		}
		inner(&mut paths, &self);
		paths
	}
}

impl std::fmt::Display for RoutePathTree {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		fn inner(
			node: &RoutePathTree,
			f: &mut std::fmt::Formatter<'_>,
		) -> std::fmt::Result {
			let suffix = if node.contains_endpoints() { "" } else { "*" };
			writeln!(f, "{}{suffix}", node.route)?;
			for child in &node.children {
				inner(child, f)?;
			}
			Ok(())
		}
		inner(self, f)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn children_are_sorted() {
		let mut world = World::new();
		let ent1 = world.spawn_empty().id();
		let ent2 = world.spawn_empty().id();
		let ent3 = world.spawn_empty().id();
		let paths = vec![
			(ent1, RoutePath::new("/zeta")),
			(ent2, RoutePath::new("/alpha")),
			(ent3, RoutePath::new("/beta")),
		];
		let tree = RoutePathTree::from_paths(paths);
		let child_names: Vec<_> =
			tree.children.iter().map(|c| c.route.to_string()).collect();
		child_names.xpect_eq(vec!["/alpha", "/beta", "/zeta"]);
	}

	#[test]
	fn route_path() {
		RoutePath::new("hello").to_string().xpect_eq("/hello");

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
			RoutePath::from_file_path(value)
				.unwrap()
				.to_string()
				.xpect_eq(expected);
		}
	}

	#[test]
	fn join() {
		RoutePath::new("/foo")
			.join(&RoutePath::new("/"))
			.to_string()
			.xpect_eq("/foo");
	}

	#[test]
	fn route_path_tree_from_paths() {
		let mut world = World::new();
		// Dummy entity values for testing
		let ent1 = world.spawn_empty().id();
		let ent2 = world.spawn_empty().id();
		let ent3 = world.spawn_empty().id();
		let ent4 = world.spawn_empty().id();
		let paths = vec![
			(ent1, RoutePath::new("/foo/bar")),
			(ent2, RoutePath::new("/foo/baz")),
			(ent3, RoutePath::new("/foo/qux/quux")),
			(ent4, RoutePath::new("/root")),
		];
		let tree = RoutePathTree::from_paths(paths.clone());

		// Root node
		tree.route.to_string().xpect_eq("/");
		tree.contains_endpoints().xpect_false();

		// Find child '/foo'
		let foo = tree
			.children
			.iter()
			.find(|c| c.route.to_string() == "/foo")
			.unwrap();
		foo.contains_endpoints().xpect_false();

		// 'bar' and 'baz' are endpoints under 'foo'
		let bar = foo
			.children
			.iter()
			.find(|c| c.route.to_string() == "/foo/bar")
			.unwrap();
		bar.contains_endpoints().xpect_true();
		(&bar.endpoints).xpect_eq(vec![ent1]);
		let baz = foo
			.children
			.iter()
			.find(|c| c.route.to_string() == "/foo/baz")
			.unwrap();
		baz.contains_endpoints().xpect_true();
		baz.endpoints.xpect_eq(vec![ent2]);

		// 'qux' is a directory, 'quux' is endpoint
		let qux = foo
			.children
			.iter()
			.find(|c| c.route.to_string() == "/foo/qux")
			.unwrap();
		qux.contains_endpoints().xpect_false();
		let quux = qux
			.children
			.iter()
			.find(|c| c.route.to_string() == "/foo/qux/quux")
			.unwrap();
		quux.contains_endpoints().xpect_true();
		quux.endpoints.xpect_eq(vec![ent3]);

		// 'root' endpoint
		let root = tree
			.children
			.iter()
			.find(|c| c.route.to_string() == "/root")
			.unwrap();
		root.contains_endpoints().xpect_true();
		root.endpoints.xpect_eq(vec![ent4]);
	}
}
