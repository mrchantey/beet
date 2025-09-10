use crate::prelude::*;
use beet_core::prelude::*;
use beet_utils::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::collections::VecDeque;
use std::ops::ControlFlow;
use std::path::Path;


/// A list of [`RouteSegment::Dynamic`] and [`RouteSegment::Wildcard`]
/// values extracted during path matching.
#[derive(Default, Clone, Resource, Deref, DerefMut, Reflect)]
pub struct DynSegmentMap(HashMap<String, String>);


/// Endpoints will only run if there are no trailing path segments,
/// unlike middleware which may run for multiple child paths.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
pub struct Endpoint;

/// A filter for matching routes based on path segments.
/// This is used to determine whether a handler should be invoked for a given request,
/// and whether its children should be processed.
/// Unlike [`RouteSegments`] this type contains *sections* of the full route path,
/// not nessecarily the entire path.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[reflect(Component)]
pub struct PathFilter {
	/// Segements that must match in order for the route to be valid,
	/// an empty vector means only the root path `/` is valid.
	pub segments: RouteSegments,
}


impl PathFilter {
	/// Create a new `PathFilter` with the given path which is split into segments.
	pub fn new(path: impl AsRef<Path>) -> Self {
		Self {
			segments: RouteSegments::parse(path),
		}
	}

	/// Consume a segment of the path for each segment in the filter,
	/// returning the remaining path if all segments match.
	pub fn matches(
		&self,
		dyn_map: &mut HashMap<String, String>,
		path: &mut VecDeque<String>,
	) -> ControlFlow<(), ()> {
		// if segments is empty, only the root path is valid
		if self.segments.is_empty() && !path.is_empty() {
			return ControlFlow::Break(());
		}

		// check each segment against the path
		for segment in self.segments.iter() {
			match segment.matches(dyn_map, path) {
				ControlFlow::Break(_) => {
					return ControlFlow::Break(());
				}
				ControlFlow::Continue(()) => {}
			}
		}
		ControlFlow::Continue(())
	}
}

/// Unlike [`PathFilter`] this type contains a full path to the endpoint
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, Component,
)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[cfg_attr(feature = "tokens", to_tokens(RouteSegments::_from_raw))]
pub struct RouteSegments {
	segments: Vec<PathSegment>,
	is_static: bool,
}

impl std::ops::Deref for RouteSegments {
	type Target = Vec<PathSegment>;
	fn deref(&self) -> &Self::Target { &self.segments }
}

impl RouteSegments {
	pub fn collect(
		entity: In<Entity>,
		parents: Query<&ChildOf>,
		filters: Query<&PathFilter>,
	) -> RouteSegments {
		parents
			.iter_ancestors_inclusive(*entity)
			.filter_map(|entity| filters.get(entity).ok())
			.collect::<Vec<_>>()
			.into_iter()
			.cloned()
			// reverse to start from the root
			.rev()
			.flat_map(|filter| filter.segments.segments)
			.collect::<Vec<_>>()
			.xmap(Self::new)
	}


	/// Parse a path into [`RouteSegments`]
	/// ## Panics
	/// - Panics if contains a wildcard pattern that isnt last
	pub fn parse(path: impl AsRef<Path>) -> Self {
		let segments = path
			.as_ref()
			.to_string_lossy()
			.split('/')
			.filter(|s| !s.is_empty())
			.map(PathSegment::new)
			.collect::<Vec<_>>();

		for (index, segment) in segments.iter().enumerate() {
			if matches!(segment, PathSegment::Wildcard(_))
				&& index != segments.len() - 1
			{
				panic!("Malformed Route Path: Wildcard pattern must be last");
			}
		}

		Self::new(segments)
	}

	/// Called by to_tokens, this should never be used directly
	pub fn _from_raw(segments: Vec<PathSegment>, is_static: bool) -> Self {
		Self {
			segments,
			is_static,
		}
	}

	pub fn new(segments: Vec<PathSegment>) -> Self {
		let is_static = segments.iter().all(|segment| segment.is_static());
		Self {
			segments,
			is_static,
		}
	}
	/// Returns true if all segments are a [`PathSegment::Static`]
	pub fn is_static(&self) -> bool { self.is_static }


	pub fn annotated_route_path(&self) -> RoutePath {
		self.segments
			.iter()
			.map(|segment| segment.to_string_annotated())
			.collect::<Vec<_>>()
			.join("/")
			.xmap(RoutePath::new)
	}
}
impl Default for RouteSegments {
	fn default() -> Self { Self::new(Vec::new()) }
}

/// A segment of a route path, stripped of:
/// - leading & trailing slashes `/`
/// - dynamic prefixes `:`
/// - wildcard prefixes `*`
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum PathSegment {
	/// A static segment, the `foo` in `/foo`
	Static(String),
	/// A dynamic segment, the `foo` in `/:foo`
	Dynamic(String),
	/// A wildcard segment, the `foo` in `/*foo`
	Wildcard(String),
}

impl PathSegment {
	/// Parses a segment from a string, determining if it is static, dynamic, or wildcard.
	///
	/// ## Panics
	/// - Panics if the segment is empty after trimming leading and trailing slashes.
	/// - Panics if the segment contains internal slashes '/'
	pub fn new(segment: impl AsRef<str>) -> Self {
		let segment = segment.as_ref();
		// trim leading and trailing slashes
		let trimmed = segment.trim_matches('/');
		if trimmed.is_empty() {
			panic!("PathSegment cannot be empty");
		} else if trimmed.contains('/') {
			panic!("PathSegment cannot contain internal slashes: {}", segment);
		} else if trimmed.starts_with(':') {
			Self::Dynamic(trimmed[1..].to_string())
		} else if trimmed.starts_with('*') {
			Self::Wildcard(trimmed[1..].to_string())
		} else {
			Self::Static(trimmed.to_string())
		}
	}
	/// Uses conventions of `:` and `*` to annotate non static segments
	pub fn to_string_annotated(&self) -> String {
		match self {
			Self::Static(val) => val.clone(),
			Self::Dynamic(val) => format!(":{}", val),
			Self::Wildcard(val) => format!("*{}", val),
		}
	}

	/// Attempts to match the segment against a path,
	/// returning the remaining path if it matches.
	pub fn matches(
		&self,
		dyn_map: &mut HashMap<String, String>,
		path: &mut VecDeque<String>,
	) -> ControlFlow<()> {
		match (self, path.pop_front()) {
			// static match, continue with remaining path
			(PathSegment::Static(val), Some(other)) if val == &other => {
				ControlFlow::Continue(())
			}
			// dynamic will always match, continue with remaining path
			(PathSegment::Dynamic(key), Some(value)) => {
				dyn_map.insert(key.clone(), value);
				ControlFlow::Continue(())
			}
			// wildcard consumes the rest of the path, continue with empty path
			(PathSegment::Wildcard(key), Some(mut value)) => {
				// consume rest of path
				while let Some(next) = path.pop_front() {
					value.push('/');
					value.push_str(&next);
				}
				dyn_map.insert(key.clone(), value);
				ControlFlow::Continue(())
			}
			// only a wildcard permits an empty path
			(PathSegment::Wildcard(key), None) => {
				dyn_map.insert(key.clone(), "".to_string());
				ControlFlow::Continue(())
			}
			// break if empty path or no matching static
			(PathSegment::Static(_) | PathSegment::Dynamic(_), _) => {
				ControlFlow::Break(())
			}
		}
	}
	pub fn is_static(&self) -> bool {
		match self {
			PathSegment::Static(_) => true,
			_ => false,
		}
	}

	pub fn as_str(&self) -> &str { self.as_ref() }
}

impl AsRef<str> for PathSegment {
	fn as_ref(&self) -> &str {
		match self {
			PathSegment::Static(s) => s,
			PathSegment::Dynamic(s) => s,
			PathSegment::Wildcard(s) => s,
		}
	}
}

impl From<&str> for PathSegment {
	fn from(value: &str) -> Self { Self::new(value) }
}
impl From<String> for PathSegment {
	fn from(value: String) -> Self { Self::new(value) }
}
/// Print the segment as-is without dynamic and wildcard annotations
impl std::fmt::Display for PathSegment {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			PathSegment::Static(s) => write!(f, "{}", s),
			PathSegment::Dynamic(s) => write!(f, "{}", s),
			PathSegment::Wildcard(s) => write!(f, "{}", s),
		}
	}
}

pub fn route_path_queue(path: &str) -> VecDeque<String> {
	path.split('/')
		.filter(|s| !s.is_empty())
		.map(|s| s.to_string())
		.collect::<VecDeque<_>>()
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::platform::collections::HashMap;
	use std::ops::ControlFlow;
	use sweet::prelude::*;

	fn expect_segment(filter: &str, request: &str) -> ControlFlow<(), ()> {
		PathFilter::new(filter)
			.matches(&mut Default::default(), &mut route_path_queue(request))
	}

	fn run_with_map(
		filter: &str,
		request: &str,
	) -> (ControlFlow<(), ()>, HashMap<String, String>) {
		let mut captured: HashMap<String, String> = Default::default();
		let mut path_parts = route_path_queue(request);
		let flow =
			PathFilter::new(filter).matches(&mut captured, &mut path_parts);
		(flow, captured)
	}

	#[test]
	fn root() {
		expect_segment("/", "/").xpect_continue();
		expect_segment("", "").xpect_continue();
		expect_segment("", "/").xpect_continue();
		expect_segment("/", "").xpect_continue();
		expect_segment("/", "/foo").xpect_break();

		for (filter, request) in [("/", "/"), ("", ""), ("", "/"), ("/", "")] {
			let (_flow, map) = run_with_map(filter, request);
			map.is_empty().xpect_true();
		}
	}
	#[test]
	fn static_path() {
		expect_segment("/foobar", "foobar").xpect_continue();
		expect_segment("foobar", "/foobar").xpect_continue();
		expect_segment("foo/bar", "foo/bar").xpect_continue();
		expect_segment("foo", "foo/bar").xpect_continue();
		expect_segment("foo/bar", "foo").xpect_break();
		expect_segment("/", "/foo").xpect_break();

		for (filter, request) in [
			("/foobar", "foobar"),
			("foobar", "/foobar"),
			("foo/bar", "foo/bar"),
			("foo", "foo/bar"),
		] {
			let (_flow, map) = run_with_map(filter, request);
			map.is_empty().xpect_true();
		}
	}
	#[test]
	fn dynamic_path() {
		expect_segment("/:foo", "bar").xpect_continue();
		expect_segment("/:foo", "/bar").xpect_continue();
		expect_segment("/:foo/:baz", "bar/baz").xpect_continue();
		expect_segment("/:foo/:baz", "/bar/baz").xpect_continue();
		expect_segment("/:foo", "bar/baz").xpect_continue();
		expect_segment("/:foo/:baz", "bar").xpect_break();
		expect_segment("/:foo", "").xpect_break();

		let (_flow, map) = run_with_map("/:foo", "bar");
		map.get("foo").cloned().xpect_eq(Some("bar".to_string()));
		map.len().xpect_eq(1);

		let (_flow, map) = run_with_map("/:foo", "/bar");
		map.get("foo").cloned().xpect_eq(Some("bar".to_string()));
		map.len().xpect_eq(1);

		let (_flow, map) = run_with_map("/:foo/:baz", "bar/baz");
		map.get("foo").cloned().xpect_eq(Some("bar".to_string()));
		map.get("baz").cloned().xpect_eq(Some("baz".to_string()));
		map.len().xpect_eq(2);

		let (_flow, map) = run_with_map("/:foo/:baz", "/bar/baz");
		map.get("foo").cloned().xpect_eq(Some("bar".to_string()));
		map.get("baz").cloned().xpect_eq(Some("baz".to_string()));
		map.len().xpect_eq(2);

		let (_flow, map) = run_with_map("/:foo", "bar/baz");
		map.get("foo").cloned().xpect_eq(Some("bar".to_string()));
		map.len().xpect_eq(1);
	}
	#[test]
	fn wildcard_path() {
		expect_segment("/*foo", "bar").xpect_continue();
		expect_segment("/*foo", "/bar").xpect_continue();
		expect_segment("/*foo", "bar/baz").xpect_continue();
		expect_segment("/*foo", "/bar/baz").xpect_continue();
		expect_segment("foo/*bar", "foo/bar/baz").xpect_continue();
		expect_segment("foo/*bar", "foo").xpect_continue();
		expect_segment("foo/*bar", "bar").xpect_break();
		expect_segment("/*foo", "").xpect_continue();

		let (_flow, map) = run_with_map("/*foo", "bar");
		map.get("foo").cloned().xpect_eq(Some("bar".to_string()));
		map.len().xpect_eq(1);

		let (_flow, map) = run_with_map("/*foo", "bar/baz");
		map.get("foo")
			.cloned()
			.xpect_eq(Some("bar/baz".to_string()));
		map.len().xpect_eq(1);

		let (_flow, map) = run_with_map("/*foo", "/bar/baz");
		map.get("foo")
			.cloned()
			.xpect_eq(Some("bar/baz".to_string()));
		map.len().xpect_eq(1);

		let (_flow, map) = run_with_map("foo/*bar", "foo/bar/baz");
		map.get("bar")
			.cloned()
			.xpect_eq(Some("bar/baz".to_string()));
		map.len().xpect_eq(1);

		let (_flow, map) = run_with_map("foo/*bar", "foo");
		map.get("bar").cloned().xpect_eq(Some("".to_string()));
		map.len().xpect_eq(1);

		let (_flow, map) = run_with_map("/*foo", "");
		map.get("foo").cloned().xpect_eq(Some("".to_string()));
		map.len().xpect_eq(1);
	}
}
