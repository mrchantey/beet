use beet_core::prelude::*;
use beet_net::prelude::*;
use std::collections::VecDeque;
use std::path::Path;
use thiserror::Error;

use crate::types::RouteQuery;


/// Represents the next part of the route pattern.
/// All ancestor [`RoutePartial`] will be prepended when determining the route pattern
/// at this point in the tree.
/// This is used to determine whether a handler should be invoked for a given request,
/// and whether its children should be processed.
#[derive(Debug, Clone, Deref, DerefMut, Component, Reflect)]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[reflect(Component)]
pub struct RoutePartial {
	/// Segements that must match in order for the route to be valid,
	/// an empty vector means only the root path `/` is valid.
	pub segments: Vec<RouteSegment>,
}

impl RoutePartial {
	/// Create a new `RoutePartial` with the given path which is split into segments.
	pub fn new(path: impl AsRef<Path>) -> Self { Self::parse(path).unwrap() }
	pub fn parse(path: impl AsRef<Path>) -> Result<Self> {
		Self {
			segments: RoutePattern::new(path)?.segments,
		}
		.xok()
	}

	pub fn from_segments(segments: Vec<RouteSegment>) -> Self {
		Self { segments }
	}
}

/// A completed sequence of [`RouteSegment`] for some point in the route tree,
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[cfg_attr(feature = "tokens", to_tokens(RouteSegments::_from_raw))]
pub struct RoutePattern {
	/// The complete sequence of segments
	segments: Vec<RouteSegment>,
	/// Is true if all segments are [`RouteSegment::Static]
	is_static: bool,
}

impl std::ops::Deref for RoutePattern {
	type Target = Vec<RouteSegment>;
	fn deref(&self) -> &Self::Target { &self.segments }
}

impl RoutePattern {
	/// Parse a path into [`RouteSegments`]
	/// ## Errors
	/// - Errors if path contains a wildcard pattern that isnt last
	pub fn new(path: impl AsRef<Path>) -> Result<Self> {
		path.as_ref()
			.to_string_lossy()
			.split('/')
			.filter(|s| !s.is_empty())
			.map(RouteSegment::new)
			.collect::<Vec<_>>()
			.xmap(Self::from_segments)
	}

	/// Parse segments into a [`RoutePattern`]
	/// ## Errors
	/// - Errors if path contains a wildcard pattern that isnt last
	pub fn from_segments(segments: Vec<RouteSegment>) -> Result<Self> {
		let is_static = segments.iter().all(|segment| segment.is_static());
		for (index, segment) in segments.iter().enumerate() {
			if matches!(segment, RouteSegment::Wildcard(_))
				&& index != segments.len() - 1
			{
				bevybail!(
					"Malformed Route Path: Wildcard pattern must be last"
				);
			}
		}

		Self {
			segments,
			is_static,
		}
		.xok()
	}

	pub fn collect(
		entity: In<Entity>,
		query: RouteQuery,
	) -> Result<RoutePattern> {
		query.route_pattern(entity.0)
	}


	/// Called by to_tokens, this should never be used directly
	pub fn _from_raw(segments: Vec<RouteSegment>, is_static: bool) -> Self {
		Self {
			segments,
			is_static,
		}
	}

	/// Returns true if all segments are a [`RouteSegment::Static`]
	pub fn is_static(&self) -> bool { self.is_static }

	/// Convert the segments to a [`RoutePath`] using annotations for dynamic segments,
	/// ie `/foo/:bar/*bazz`
	pub fn annotated_route_path(&self) -> RoutePath {
		self.segments
			.iter()
			.map(|segment| segment.to_string_annotated())
			.collect::<Vec<_>>()
			.join("/")
			.xmap(RoutePath::new)
	}
	/// Consume a segment of the path for each segment in the filter,
	/// returning the remaining path if all segments match.
	pub fn parse_path(
		&self,
		path: &RoutePath,
	) -> Result<RouteMatch, RouteMatchError> {
		let mut remaining_path = path
			.to_string_lossy()
			.split('/')
			.filter(|s| !s.is_empty())
			.map(|s| s.to_string())
			.collect::<Vec<_>>()
			.xmap(VecDeque::from);

		let mut dyn_map = default();
		// check each segment against the path
		for segment in self.segments.iter() {
			segment.parse_parts(&mut dyn_map, &mut remaining_path)?;
		}
		RouteMatch {
			remaining_path,
			dyn_map,
		}
		.xok()
	}
}

impl std::fmt::Display for RoutePattern {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.annotated_route_path())
	}
}

/// A segment of a route path, stripped of:
/// - leading & trailing slashes `/`
/// - dynamic prefixes `:`
/// - wildcard prefixes `*`
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum RouteSegment {
	/// A static segment, the `foo` in `/foo`
	Static(String),
	/// A dynamic segment, the `foo` in `/:foo`
	Dynamic(String),
	/// A wildcard segment, the `foo` in `/*foo`
	Wildcard(String),
}

/// The result of a successful route match,
/// containing the remaining unmatched path parts and a map of dynamic segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteMatch {
	pub remaining_path: VecDeque<String>,
	pub dyn_map: HashMap<String, String>,
}
impl RouteMatch {
	/// Returns true if there is no remaining path to match
	pub fn exact_match(&self) -> bool { self.remaining_path.is_empty() }
}

pub type RouteMatchResult = Result<RouteMatch, RouteMatchError>;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RouteMatchError {
	/// A [`RouteSegment::Static`] did not match its corresponding [`RoutePath`] part.
	#[error(
		"a static segment '{segment}' did not match its corresponding path part '{path}'"
	)]
	InvalidStatic { segment: String, path: String },
	/// A [`RouteSegment::Static`] did not match its corresponding [`RoutePath`] part.
	#[error(
		"a segment '{segment}' expected at least one path segment, but it was empty"
	)]
	EmptyPath { segment: RouteSegment },
}

impl RouteSegment {
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
			panic!("RouteSegment cannot be empty");
		} else if trimmed.contains('/') {
			panic!("RouteSegment cannot contain internal slashes: {}", segment);
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
	/// In the case of a wildcard all remaining parts are consumed.
	pub fn parse_parts(
		&self,
		dyn_map: &mut HashMap<String, String>,
		path: &mut VecDeque<String>,
	) -> Result<(), RouteMatchError> {
		let mut insert = |key: String, value: String| {
			if dyn_map.contains_key(&key) {
				error!(
					"Duplicate dynamic segment key: {}\nThis will result in unexpected behavior
					Please check for overlapping routes",
					key
				);
			}
			dyn_map.insert(key, value);
		};


		match (self, path.pop_front()) {
			// static match, continue with remaining path
			(RouteSegment::Static(val), Some(other)) if val == &other => Ok(()),
			// static but no match, this is an error
			(RouteSegment::Static(val), Some(other)) => {
				Err(RouteMatchError::InvalidStatic {
					segment: val.clone(),
					path: other,
				})
			}
			// dynamic will always match, continue with remaining path
			(RouteSegment::Dynamic(key), Some(value)) => {
				insert(key.clone(), value);
				Ok(())
			}
			// wildcard consumes the rest of the path, continue with empty path
			(RouteSegment::Wildcard(key), Some(mut value)) => {
				// consume rest of path
				while let Some(next) = path.pop_front() {
					value.push('/');
					value.push_str(&next);
				}
				insert(key.clone(), value);
				Ok(())
			}
			// break if empty path
			(segment, None) => Err(RouteMatchError::EmptyPath {
				segment: segment.clone(),
			}),
		}
	}
	pub fn is_static(&self) -> bool {
		match self {
			RouteSegment::Static(_) => true,
			_ => false,
		}
	}

	pub fn as_str(&self) -> &str { self.as_ref() }
}

impl AsRef<str> for RouteSegment {
	fn as_ref(&self) -> &str {
		match self {
			RouteSegment::Static(s) => s,
			RouteSegment::Dynamic(s) => s,
			RouteSegment::Wildcard(s) => s,
		}
	}
}

impl From<&str> for RouteSegment {
	fn from(value: &str) -> Self { Self::new(value) }
}
impl From<String> for RouteSegment {
	fn from(value: String) -> Self { Self::new(value) }
}
/// Print the segment as-is without dynamic and wildcard annotations
impl std::fmt::Display for RouteSegment {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			RouteSegment::Static(s) => write!(f, "{}", s),
			RouteSegment::Dynamic(s) => write!(f, "{}", s),
			RouteSegment::Wildcard(s) => write!(f, "{}", s),
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
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;

	/// match segments against a route path
	fn parse(
		segments: &str,
		route_path: &str,
	) -> Result<RouteMatch, RouteMatchError> {
		RoutePattern::new(segments)
			.unwrap()
			.parse_path(&RoutePath::new(route_path))
	}

	#[test]
	fn root() {
		parse("/", "/").xpect_ok();
		parse("", "").xpect_ok();
		parse("", "/").xpect_ok();
		parse("/", "").xpect_ok();
		parse("/", "/foo").unwrap().exact_match().xpect_false();

		for (filter, request) in [("/", "/"), ("", ""), ("", "/"), ("/", "")] {
			parse(filter, request)
				.unwrap()
				.dyn_map
				.is_empty()
				.xpect_true();
		}
	}
	#[test]
	fn static_path() {
		parse("/foobar", "foobar")
			.unwrap()
			.exact_match()
			.xpect_true();
		parse("foobar", "/foobar")
			.unwrap()
			.exact_match()
			.xpect_true();
		parse("foo/bar", "foo/bar")
			.unwrap()
			.exact_match()
			.xpect_true();
		parse("foo", "foo/bar").unwrap().exact_match().xpect_false();
		parse("/", "/foo").unwrap().exact_match().xpect_false();
		parse("foo/bar", "foo").xpect_err();

		for (filter, request) in [
			("/foobar", "foobar"),
			("foobar", "/foobar"),
			("foo/bar", "foo/bar"),
			("foo", "foo/bar"),
		] {
			let map = parse(filter, request).unwrap().dyn_map;
			map.is_empty().xpect_true();
		}
	}
	#[test]
	fn dynamic_path() {
		parse("/:foo", "bar").xpect_ok();
		parse("/:foo", "/bar").xpect_ok();
		parse("/:foo/:baz", "bar/baz").xpect_ok();
		parse("/:foo/:baz", "/bar/baz").xpect_ok();
		parse("/:foo", "bar/baz").xpect_ok();
		parse("/:foo/:baz", "bar").xpect_err();
		parse("/:foo", "").xpect_err();

		let map = parse("/:foo", "bar").unwrap().dyn_map;
		map.get("foo").cloned().xpect_eq(Some("bar".to_string()));
		map.len().xpect_eq(1);

		let map = parse("/:foo", "/bar").unwrap().dyn_map;
		map.get("foo").cloned().xpect_eq(Some("bar".to_string()));
		map.len().xpect_eq(1);

		let map = parse("/:foo/:baz", "bar/baz").unwrap().dyn_map;
		map.get("foo").cloned().xpect_eq(Some("bar".to_string()));
		map.get("baz").cloned().xpect_eq(Some("baz".to_string()));
		map.len().xpect_eq(2);

		let map = parse("/:foo/:baz", "/bar/baz").unwrap().dyn_map;
		map.get("foo").cloned().xpect_eq(Some("bar".to_string()));
		map.get("baz").cloned().xpect_eq(Some("baz".to_string()));
		map.len().xpect_eq(2);

		let map = parse("/:foo", "bar/baz").unwrap().dyn_map;
		map.get("foo").cloned().xpect_eq(Some("bar".to_string()));
		map.len().xpect_eq(1);
	}
	#[test]
	fn wildcard_path() {
		parse("/*foo", "bar").xpect_ok();
		parse("/*foo", "/bar").xpect_ok();
		parse("/*foo", "bar/baz").xpect_ok();
		parse("/*foo", "/bar/baz").xpect_ok();
		parse("foo/*bar", "foo/bar/baz").xpect_ok();
		// missing final segment
		parse("foo/*bar", "foo").xpect_eq(Err(RouteMatchError::EmptyPath {
			segment: RouteSegment::new("*bar"),
		}));
		parse("foo/*bar", "bar").xpect_err();
		parse("/*foo", "").xpect_err();

		let map = parse("/*foo", "bar").unwrap().dyn_map;
		map.get("foo").cloned().xpect_eq(Some("bar".to_string()));
		map.len().xpect_eq(1);

		let map = parse("/*foo", "bar/baz").unwrap().dyn_map;
		map.get("foo")
			.cloned()
			.xpect_eq(Some("bar/baz".to_string()));
		map.len().xpect_eq(1);

		let map = parse("/*foo", "/bar/baz").unwrap().dyn_map;
		map.get("foo")
			.cloned()
			.xpect_eq(Some("bar/baz".to_string()));
		map.len().xpect_eq(1);

		let map = parse("foo/*bar", "foo/bar/baz").unwrap().dyn_map;
		map.get("bar")
			.cloned()
			.xpect_eq(Some("bar/baz".to_string()));
		map.len().xpect_eq(1);
	}
}
