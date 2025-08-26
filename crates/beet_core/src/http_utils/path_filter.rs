#[cfg(feature = "tokens")]
use crate::as_beet::*;
use crate::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use http::request::Parts;
use std::collections::VecDeque;
use std::ops::ControlFlow;
use std::path::Path;


/// Endpoints will only run if there are no trailing path segments,
/// unlike middleware which may run for multiple child paths.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
pub struct Endpoint;

/// A filter for matching routes based on path segments.
/// This is used to determine whether a handler should be invoked for a given request,
/// and whether its children should be processed.
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
		mut parts: RouteParts,
	) -> ControlFlow<(), RouteParts> {
		// if segments is empty, only the root path is valid
		if self.segments.is_empty() && !parts.path.is_empty() {
			return ControlFlow::Break(());
		}

		// check each segment against the path
		for segment in self.segments.iter() {
			let next = parts.path.pop_front();
			match segment.matches(next.as_ref()) {
				ControlFlow::Break(_) => {
					return ControlFlow::Break(());
				}
				ControlFlow::Continue(()) => {
					if matches!(segment, PathSegment::Wildcard(_)) {
						// wildcards match and consume all remaining segments
						parts.path = Default::default();
						return ControlFlow::Continue(parts);
					}
				}
			}
		}
		ControlFlow::Continue(parts)
	}
}


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
				panic!("Wildcard pattern must be last");
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
	pub fn matches(&self, segment: Option<&String>) -> ControlFlow<()> {
		match (self, segment) {
			// static match, continue with remaining path
			(PathSegment::Static(val), Some(other)) if val == other => {
				ControlFlow::Continue(())
			}
			// dynamic will always match, continue with remaining path
			(PathSegment::Dynamic(_), Some(_)) => ControlFlow::Continue(()),
			// wildcard consumes the rest of the path, continue with empty path
			(PathSegment::Wildcard(_), Some(_)) => {
				ControlFlow::Continue(Default::default())
			}
			// only a wildcard permits an empty path
			(PathSegment::Wildcard(_), None) => {
				ControlFlow::Continue(Default::default())
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

/// A [`RoutePath`] split into segments for easier matching,
/// where each segment is guaranteed to be:
/// - non-empty
/// - not contain internal slashes `/`
#[derive(Debug, Default, Clone)]
pub struct RouteParts {
	pub(super) method: HttpMethod,
	/// Non-empty segments of the path,
	path: VecDeque<String>,
}

impl RouteParts {
	/// Create a new `RouteParts` from a path without a query and method.
	pub fn new(path: &str, method: HttpMethod) -> Self {
		Self {
			method,
			path: path
				.split('/')
				.filter(|s| !s.is_empty())
				.map(|s| s.to_string())
				.collect::<VecDeque<_>>(),
		}
	}
	/// Parse the uri
	pub fn from_parts(parts: &Parts) -> Self {
		Self::new(parts.uri.path(), parts.method.clone().into())
	}
	pub fn method(&self) -> HttpMethod { self.method }
	pub fn path(&self) -> &VecDeque<String> { &self.path }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use std::ops::ControlFlow;
	use sweet::prelude::*;

	fn expect_segment(
		filter: &str,
		request: &str,
	) -> Matcher<ControlFlow<(), RouteParts>> {
		PathFilter::new(filter)
			.matches(RouteParts::new(request, HttpMethod::Get))
			.xpect()
	}
	#[test]
	fn root() {
		expect_segment("/", "/").to_continue();
		expect_segment("", "").to_continue();
		expect_segment("", "/").to_continue();
		expect_segment("/", "").to_continue();
		expect_segment("/", "/foo").to_break();
	}
	#[test]
	fn static_path() {
		expect_segment("/foobar", "foobar").to_continue();
		expect_segment("foobar", "/foobar").to_continue();
		expect_segment("foo/bar", "foo/bar").to_continue();
		expect_segment("foo", "foo/bar").to_continue();
		expect_segment("foo/bar", "foo").to_break();
		expect_segment("/", "/foo").to_break();
	}
	#[test]
	fn dynamic_path() {
		expect_segment("/:foo", "bar").to_continue();
		expect_segment("/:foo", "/bar").to_continue();
		expect_segment("/:foo/:baz", "bar/baz").to_continue();
		expect_segment("/:foo/:baz", "/bar/baz").to_continue();
		expect_segment("/:foo", "bar/baz").to_continue();
		expect_segment("/:foo/:baz", "bar").to_break();
		expect_segment("/:foo", "").to_break();
	}
	#[test]
	fn wildcard_path() {
		expect_segment("/*foo", "bar").to_continue();
		expect_segment("/*foo", "/bar").to_continue();
		expect_segment("/*foo", "bar/baz").to_continue();
		expect_segment("/*foo", "/bar/baz").to_continue();
		expect_segment("foo/*bar", "foo/bar/baz").to_continue();
		expect_segment("foo/*bar", "foo").to_continue();
		expect_segment("foo/*bar", "bar").to_break();
		expect_segment("/*foo", "").to_continue();
	}
}
