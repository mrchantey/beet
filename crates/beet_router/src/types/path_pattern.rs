//! Pattern matching features loosely based on the [URL Pattern API](https://developer.mozilla.org/en-US/docs/Web/API/URL_Pattern_API)
use beet_core::prelude::*;
use beet_net::prelude::*;
use std::collections::VecDeque;
use std::path::Path;
use thiserror::Error;

use crate::types::RouteQuery;

/// Modifier for path pattern segments, aligned with URL Pattern API.
///
/// These modifiers control whether a segment is static or dynamic, and for dynamic
/// segments they control cardinality and greediness:
/// - `Static` - exact string match (default)
/// - `Required` / `Optional` - match exactly one or zero-to-one segments (non-greedy)
/// - `OneOrMore` / `ZeroOrMore` - match one+ or zero+ segments (greedy, consumes rest of path)
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum PathPatternModifier {
	/// Default - static segment, exact string match
	#[default]
	Static,
	/// Exactly one segment required (`:param`)
	Required,
	/// Zero or one segment (`:param?`)
	Optional,
	/// One or more segments, greedy (`:param+` or `*param`)
	OneOrMore,
	/// Zero or more segments, greedy (`:param*` or `*param?`)
	ZeroOrMore,
}

impl PathPatternModifier {
	/// Returns true if this modifier allows zero matches
	pub fn is_optional(&self) -> bool {
		matches!(self, Self::Optional | Self::ZeroOrMore)
	}

	/// Returns true if this modifier is greedy (consumes multiple segments)
	pub fn is_greedy(&self) -> bool {
		matches!(self, Self::OneOrMore | Self::ZeroOrMore)
	}

	/// Returns true if this is a static segment
	pub fn is_static(&self) -> bool { matches!(self, Self::Static) }
}

/// Represents the next part of the route pattern.
/// All ancestor [`PathPartial`] will be prepended when determining the route pattern
/// at this point in the tree.
/// This is used to determine whether a handler should be invoked for a given request,
/// and whether its children should be processed.
#[derive(Debug, Clone, Deref, DerefMut, Component, Reflect)]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[reflect(Component)]
pub struct PathPartial {
	/// Segements that must match in order for the route to be valid,
	/// an empty vector means only the root path `/` is valid.
	pub segments: Vec<PathPatternSegment>,
}

impl PathPartial {
	/// Create a new `PathPartial` with the given path which is split into segments.
	pub fn new(path: impl AsRef<Path>) -> Self { Self::parse(path).unwrap() }
	pub fn parse(path: impl AsRef<Path>) -> Result<Self> {
		Self {
			segments: PathPattern::new(path)?.segments,
		}
		.xok()
	}

	pub fn from_segments(segments: Vec<PathPatternSegment>) -> Self {
		Self { segments }
	}
}

/// A completed sequence of [`PathPatternSegment`] for some point in the route tree.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[cfg_attr(feature = "tokens", to_tokens(PathPatternSegments::_from_raw))]
pub struct PathPattern {
	/// The complete sequence of segments
	segments: Vec<PathPatternSegment>,
	/// Is true if all segments are static
	is_static: bool,
}

impl std::ops::Deref for PathPattern {
	type Target = Vec<PathPatternSegment>;
	fn deref(&self) -> &Self::Target { &self.segments }
}

impl PathPattern {
	/// Parse a path into [`PathPatternSegments`]
	/// ## Errors
	/// - Errors if path contains a greedy pattern that isnt last
	pub fn new(path: impl AsRef<Path>) -> Result<Self> {
		path.as_ref()
			.to_string_lossy()
			.split('/')
			.filter(|s| !s.is_empty())
			.map(PathPatternSegment::new)
			.collect::<Vec<_>>()
			.xmap(Self::from_segments)
	}

	/// Parse segments into a [`PathPattern`]
	/// ## Errors
	/// - Errors if path contains a greedy pattern that isnt last
	pub fn from_segments(segments: Vec<PathPatternSegment>) -> Result<Self> {
		let is_static = segments.iter().all(|segment| segment.is_static());
		for (index, segment) in segments.iter().enumerate() {
			if segment.is_greedy() && index != segments.len() - 1 {
				bevybail!(
					"Malformed Route Path: Greedy pattern (wildcard/repeating) must be last"
				);
			}
		}

		Self {
			segments,
			is_static,
		}
		.xok()
	}

	/// [`Self::Collect`] represented as a bevy system
	pub fn collect_system(
		entity: In<Entity>,
		query: RouteQuery,
	) -> Result<PathPattern> {
		Self::collect(*entity, &query)
	}

	pub fn collect(entity: Entity, query: &RouteQuery) -> Result<PathPattern> {
		query
			.parents
			// get every PathFilter in ancestors
			.iter_ancestors_inclusive(entity)
			.filter_map(|entity| query.path_partials.get(entity).ok())
			.collect::<Vec<_>>()
			.into_iter()
			.cloned()
			// reverse to start from the root
			.rev()
			// extract the segments
			.flat_map(|partial| partial.segments)
			.collect::<Vec<_>>()
			.xmap(Self::from_segments)
	}


	/// Called by to_tokens, this should never be used directly
	pub fn _from_raw(
		segments: Vec<PathPatternSegment>,
		is_static: bool,
	) -> Self {
		Self {
			segments,
			is_static,
		}
	}

	/// Returns true if all segments are static
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
	/// Consume a segment of the path for each segment in [`Self::segments`],
	/// returning the remaining path if all segments match.
	/// Adjacent slashes are preserved as empty strings, allowing greedy segments
	/// to reconstruct paths with double slashes like `foo//bar/baz.rs`.
	pub fn parse_path(
		&self,
		path: &Vec<String>,
	) -> Result<PathMatch, RouteMatchError> {
		let mut remaining_path = VecDeque::from(path.clone());

		let mut dyn_map = default();
		// check each segment against the path
		for segment in self.segments.iter() {
			segment.parse_parts(&mut dyn_map, &mut remaining_path)?;
		}
		PathMatch {
			remaining_path,
			dyn_map,
		}
		.xok()
	}
}

impl std::fmt::Display for PathPattern {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.annotated_route_path())
	}
}

/// A segment of a route path.
///
/// Contains a name and a modifier that determines how the segment matches:
/// - Static segments match exactly
/// - Dynamic segments capture values from the path
///
/// ## Syntax
///
/// Aligned with URL Pattern API conventions:
/// - `foo` - Static segment, exact match
/// - `:foo` - Dynamic segment, matches exactly one path segment
/// - `:foo?` - Optional dynamic, matches zero or one segment
/// - `:foo+` - Repeating dynamic, matches one or more segments (greedy)
/// - `:foo*` - Optional repeating, matches zero or more segments (greedy)
/// - `*foo` - Shorthand for `:foo+` (one or more, greedy)
/// - `*foo?` - Shorthand for `:foo*` (zero or more, greedy)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct PathPatternSegment {
	/// The segment name (without prefixes/suffixes like `:`, `*`, `?`, `+`)
	name: String,
	/// The modifier controlling how this segment matches
	modifier: PathPatternModifier,
}

/// The result of a successful route match,
/// containing the remaining unmatched path parts and a map of dynamic segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathMatch {
	pub remaining_path: VecDeque<String>,
	/// Dynamic segment values. For duplicate and greedy segments, each matched path segment
	/// is stored as a separate value.
	pub dyn_map: MultiMap<String, String>,
}
impl PathMatch {
	/// Returns true if there is no remaining path to match
	pub fn exact_match(&self) -> bool { self.remaining_path.is_empty() }
}

pub type RouteMatchResult = Result<PathMatch, RouteMatchError>;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RouteMatchError {
	/// A static segment did not match its corresponding [`RoutePath`] part.
	#[error(
		"a static segment '{segment}' did not match its corresponding path part '{path}'"
	)]
	InvalidStatic { segment: String, path: String },
	/// A segment expected at least one path part but the path was empty.
	#[error(
		"a segment '{segment}' expected at least one path segment, but it was empty"
	)]
	EmptyPath { segment: PathPatternSegment },
}

impl PathPatternSegment {
	/// Parses a segment from a string, determining if it is static or dynamic,
	/// and extracting any modifiers.
	///
	/// ## Syntax
	/// - `foo` - Static segment
	/// - `:foo` - Dynamic, required (one segment)
	/// - `:foo?` - Dynamic, optional (zero or one segment)
	/// - `:foo+` - Dynamic, one or more (greedy)
	/// - `:foo*` - Dynamic, zero or more (greedy)
	/// - `*foo` - Shorthand for `:foo+`
	/// - `*foo?` - Shorthand for `:foo*`
	///
	/// ## Panics
	/// - Panics if the segment is empty after trimming leading and trailing slashes.
	/// - Panics if the segment contains internal slashes '/'
	pub fn new(segment: impl AsRef<str>) -> Self {
		let segment = segment.as_ref();
		// trim leading and trailing slashes
		let trimmed = segment.trim_matches('/');
		if trimmed.is_empty() {
			panic!("PathPatternSegment cannot be empty");
		} else if trimmed.contains('/') {
			panic!(
				"PathPatternSegment cannot contain internal slashes: {}",
				segment
			);
		} else if trimmed.starts_with('*') {
			// Wildcard shorthand: *foo or *foo?
			let rest = &trimmed[1..];
			if let Some(name) = rest.strip_suffix('?') {
				// *foo? = zero or more (optional greedy)
				Self {
					name: name.to_string(),
					modifier: PathPatternModifier::ZeroOrMore,
				}
			} else {
				// *foo = one or more (required greedy)
				Self {
					name: rest.to_string(),
					modifier: PathPatternModifier::OneOrMore,
				}
			}
		} else if trimmed.starts_with(':') {
			// Dynamic segment with possible modifier
			let rest = &trimmed[1..];
			if let Some(name) = rest.strip_suffix('?') {
				Self {
					name: name.to_string(),
					modifier: PathPatternModifier::Optional,
				}
			} else if let Some(name) = rest.strip_suffix('+') {
				Self {
					name: name.to_string(),
					modifier: PathPatternModifier::OneOrMore,
				}
			} else if let Some(name) = rest.strip_suffix('*') {
				Self {
					name: name.to_string(),
					modifier: PathPatternModifier::ZeroOrMore,
				}
			} else {
				Self {
					name: rest.to_string(),
					modifier: PathPatternModifier::Required,
				}
			}
		} else {
			Self {
				name: trimmed.to_string(),
				modifier: PathPatternModifier::Static,
			}
		}
	}

	/// Creates a static segment
	pub fn static_segment(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			modifier: PathPatternModifier::Static,
		}
	}

	/// Creates a dynamic segment with the given modifier
	pub fn dynamic(
		name: impl Into<String>,
		modifier: PathPatternModifier,
	) -> Self {
		Self {
			name: name.into(),
			modifier,
		}
	}

	/// Creates a required dynamic segment (`:param`)
	pub fn dynamic_required(name: impl Into<String>) -> Self {
		Self::dynamic(name, PathPatternModifier::Required)
	}

	/// Creates an optional dynamic segment (`:param?`)
	pub fn dynamic_optional(name: impl Into<String>) -> Self {
		Self::dynamic(name, PathPatternModifier::Optional)
	}

	/// Creates a one-or-more greedy segment (`:param+` or `*param`)
	pub fn one_or_more(name: impl Into<String>) -> Self {
		Self::dynamic(name, PathPatternModifier::OneOrMore)
	}

	/// Creates a zero-or-more greedy segment (`:param*` or `*param?`)
	pub fn zero_or_more(name: impl Into<String>) -> Self {
		Self::dynamic(name, PathPatternModifier::ZeroOrMore)
	}

	/// Uses conventions of `:`, `*`, `?`, `+` to annotate segments
	pub fn to_string_annotated(&self) -> String {
		match self.modifier {
			PathPatternModifier::Static => self.name.clone(),
			PathPatternModifier::Required => format!(":{}", self.name),
			PathPatternModifier::Optional => format!(":{}?", self.name),
			PathPatternModifier::OneOrMore => format!("*{}", self.name),
			PathPatternModifier::ZeroOrMore => format!("*{}?", self.name),
		}
	}

	/// Returns true if this segment is greedy (consumes multiple path segments)
	pub fn is_greedy(&self) -> bool { self.modifier.is_greedy() }

	/// Attempts to match the segment against a path,
	/// returning the remaining path if it matches.
	///
	/// For greedy segments (OneOrMore, ZeroOrMore), all remaining parts are consumed
	/// and stored as separate values in the multimap.
	pub fn parse_parts(
		&self,
		dyn_map: &mut MultiMap<String, String>,
		path: &mut VecDeque<String>,
	) -> Result<(), RouteMatchError> {
		match (&self.modifier, path.pop_front()) {
			// Static match - must match exactly
			(PathPatternModifier::Static, Some(other))
				if self.name == other =>
			{
				Ok(())
			}
			(PathPatternModifier::Static, Some(other)) => {
				Err(RouteMatchError::InvalidStatic {
					segment: self.name.clone(),
					path: other,
				})
			}
			(PathPatternModifier::Static, None) => {
				Err(RouteMatchError::EmptyPath {
					segment: self.clone(),
				})
			}

			// Dynamic Required - must match exactly one non-empty segment
			(PathPatternModifier::Required, Some(value)) => {
				if value.is_empty() {
					Err(RouteMatchError::InvalidStatic {
						segment: "dynamic segment".to_string(),
						path: value,
					})
				} else {
					dyn_map.insert(self.name.clone(), value);
					Ok(())
				}
			}
			(PathPatternModifier::Required, None) => {
				Err(RouteMatchError::EmptyPath {
					segment: self.clone(),
				})
			}

			// Dynamic Optional - matches zero or one segment
			(PathPatternModifier::Optional, Some(value)) => {
				if value.is_empty() {
					// Empty string from adjacent slash - treat as no match, put it back
					path.push_front(value);
					dyn_map.insert_key(self.name.clone());
				} else {
					dyn_map.insert(self.name.clone(), value);
				}
				Ok(())
			}
			(PathPatternModifier::Optional, None) => {
				dyn_map.insert_key(self.name.clone());
				Ok(())
			}

			// Dynamic OneOrMore - greedy, must have at least one segment
			(PathPatternModifier::OneOrMore, Some(value)) => {
				// Insert first segment
				dyn_map.insert(self.name.clone(), value);
				// Insert remaining segments separately
				while let Some(next) = path.pop_front() {
					dyn_map.insert(self.name.clone(), next);
				}
				Ok(())
			}
			(PathPatternModifier::OneOrMore, None) => {
				Err(RouteMatchError::EmptyPath {
					segment: self.clone(),
				})
			}

			// Dynamic ZeroOrMore - greedy, can match empty
			(PathPatternModifier::ZeroOrMore, Some(value)) => {
				// Insert first segment
				dyn_map.insert(self.name.clone(), value);
				// Insert remaining segments separately
				while let Some(next) = path.pop_front() {
					dyn_map.insert(self.name.clone(), next);
				}
				Ok(())
			}
			(PathPatternModifier::ZeroOrMore, None) => {
				// Zero matches is valid for ZeroOrMore - create empty entry
				dyn_map.insert_key(self.name.clone());
				Ok(())
			}
		}
	}

	/// Returns true if this is a static segment
	pub fn is_static(&self) -> bool { self.modifier.is_static() }

	/// Returns the name of this segment
	pub fn as_str(&self) -> &str { &self.name }

	/// Returns the name of this segment
	pub fn name(&self) -> &str { &self.name }

	/// Returns the modifier for this segment
	pub fn modifier(&self) -> PathPatternModifier { self.modifier }
}

impl AsRef<str> for PathPatternSegment {
	fn as_ref(&self) -> &str { &self.name }
}

impl From<&str> for PathPatternSegment {
	fn from(value: &str) -> Self { Self::new(value) }
}
impl From<String> for PathPatternSegment {
	fn from(value: String) -> Self { Self::new(value) }
}
/// Print the segment name without dynamic and wildcard annotations
impl std::fmt::Display for PathPatternSegment {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.name)
	}
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
	) -> Result<PathMatch, RouteMatchError> {
		let builder = PartsBuilder::new().path_str(route_path).build();
		PathPattern::new(segments)
			.unwrap()
			.parse_path(&builder.path())
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
		// Using *foo syntax (shorthand for :foo+, OneOrMore)
		parse("/*foo", "bar").xpect_ok();
		parse("/*foo", "/bar").xpect_ok();
		parse("/*foo", "bar/baz").xpect_ok();
		parse("/*foo", "/bar/baz").xpect_ok();
		parse("foo/*bar", "foo/bar/baz").xpect_ok();
		// missing final segment - OneOrMore requires at least one
		parse("foo/*bar", "foo").xpect_eq(Err(RouteMatchError::EmptyPath {
			segment: PathPatternSegment::new("*bar"),
		}));
		parse("foo/*bar", "bar").xpect_err();
		parse("/*foo", "").xpect_err();

		let map = parse("/*foo", "bar").unwrap().dyn_map;
		map.get("foo").cloned().xpect_eq(Some("bar".to_string()));
		map.len().xpect_eq(1);

		let map = parse("/*foo", "bar/baz").unwrap().dyn_map;
		map.get_vec("foo")
			.xpect_eq(Some(&vec!["bar".to_string(), "baz".to_string()]));
		map.len().xpect_eq(1);

		let map = parse("/*foo", "/bar/baz").unwrap().dyn_map;
		map.get_vec("foo")
			.xpect_eq(Some(&vec!["bar".to_string(), "baz".to_string()]));
		map.len().xpect_eq(1);

		let map = parse("foo/*bar", "foo/bar/baz").unwrap().dyn_map;
		map.get_vec("bar")
			.xpect_eq(Some(&vec!["bar".to_string(), "baz".to_string()]));
		map.len().xpect_eq(1);
	}

	#[test]
	fn optional_wildcard_path() {
		// Using *foo? syntax (shorthand for :foo*, ZeroOrMore)
		parse("/*foo?", "bar").xpect_ok();
		parse("/*foo?", "/bar").xpect_ok();
		parse("/*foo?", "bar/baz").xpect_ok();
		// ZeroOrMore allows empty!
		parse("/*foo?", "").xpect_ok();
		parse("foo/*bar?", "foo").xpect_ok();

		// Empty match creates key with no values
		let map = parse("/*foo?", "").unwrap().dyn_map;
		map.contains_key("foo").xpect_true();
		map.get_vec("foo").xpect_eq(Some(&Vec::<String>::new()));
		map.len().xpect_eq(1);

		let map = parse("foo/*bar?", "foo").unwrap().dyn_map;
		map.contains_key("bar").xpect_true();
		map.get_vec("bar").xpect_eq(Some(&Vec::<String>::new()));
		map.len().xpect_eq(1);

		// Non-empty match stores each segment separately
		let map = parse("/*foo?", "bar/baz").unwrap().dyn_map;
		map.get_vec("foo")
			.xpect_eq(Some(&vec!["bar".to_string(), "baz".to_string()]));
		map.len().xpect_eq(1);
	}

	#[test]
	fn optional_dynamic_path() {
		// Using :foo? syntax (Optional)
		parse("/:foo?", "bar").xpect_ok();
		parse("/:foo?", "").xpect_ok();
		parse("/prefix/:foo?", "prefix").xpect_ok();
		parse("/prefix/:foo?", "prefix/value").xpect_ok();

		// Empty match creates key with no values
		let map = parse("/:foo?", "").unwrap().dyn_map;
		map.contains_key("foo").xpect_true();
		map.get_vec("foo").xpect_eq(Some(&Vec::<String>::new()));
		map.len().xpect_eq(1);

		// With prefix
		let map = parse("/prefix/:foo?", "prefix").unwrap().dyn_map;
		map.contains_key("foo").xpect_true();
		map.get_vec("foo").xpect_eq(Some(&Vec::<String>::new()));
		map.len().xpect_eq(1);

		// Non-empty match
		let map = parse("/:foo?", "bar").unwrap().dyn_map;
		map.get("foo").cloned().xpect_eq(Some("bar".to_string()));
		map.len().xpect_eq(1);
	}

	#[test]
	fn explicit_modifier_syntax() {
		// Test :foo+ syntax (explicit OneOrMore)
		parse("/:foo+", "bar").xpect_ok();
		parse("/:foo+", "bar/baz").xpect_ok();
		parse("/:foo+", "").xpect_err();

		let map = parse("/:foo+", "bar/baz").unwrap().dyn_map;
		map.get_vec("foo")
			.xpect_eq(Some(&vec!["bar".to_string(), "baz".to_string()]));

		// Test :foo* syntax (explicit ZeroOrMore)
		parse("/:foo*", "bar").xpect_ok();
		parse("/:foo*", "bar/baz").xpect_ok();
		parse("/:foo*", "").xpect_ok();

		let map = parse("/:foo*", "").unwrap().dyn_map;
		map.contains_key("foo").xpect_true();
		map.get_vec("foo").xpect_eq(Some(&Vec::<String>::new()));
	}

	#[test]
	fn adjacent_slashes() {
		// greedy segments store each part separately
		let map = parse("foo/*bar", "foo//bar/baz.rs").unwrap().dyn_map;
		map.get_vec("bar").xpect_eq(Some(&vec![
			// "".to_string(),
			"bar".to_string(),
			"baz.rs".to_string(),
		]));
		map.len().xpect_eq(1);

		let map = parse("/*file", "/bar//baz.rs").unwrap().dyn_map;
		map.get_vec("file").xpect_eq(Some(&vec![
			"bar".to_string(),
			// "".to_string(),
			"baz.rs".to_string(),
		]));
		map.len().xpect_eq(1);

		// multiple adjacent slashes stored as empty strings
		let map = parse("/*path", "foo///bar").unwrap().dyn_map;
		map.get_vec("path").xpect_eq(Some(&vec![
			"foo".to_string(),
			// "".to_string(),
			// "".to_string(),
			"bar".to_string(),
		]));
		map.len().xpect_eq(1);
	}

	#[test]
	fn segment_parsing() {
		// Static
		let seg = PathPatternSegment::new("foo");
		seg.is_static().xpect_true();
		seg.name().xpect_eq("foo");

		// Dynamic Required
		let seg = PathPatternSegment::new(":foo");
		seg.modifier().xpect_eq(PathPatternModifier::Required);
		seg.name().xpect_eq("foo");

		// Dynamic Optional
		let seg = PathPatternSegment::new(":foo?");
		seg.modifier().xpect_eq(PathPatternModifier::Optional);
		seg.name().xpect_eq("foo");

		// Dynamic OneOrMore (explicit)
		let seg = PathPatternSegment::new(":foo+");
		seg.modifier().xpect_eq(PathPatternModifier::OneOrMore);
		seg.name().xpect_eq("foo");

		// Dynamic ZeroOrMore (explicit)
		let seg = PathPatternSegment::new(":foo*");
		seg.modifier().xpect_eq(PathPatternModifier::ZeroOrMore);
		seg.name().xpect_eq("foo");

		// Wildcard shorthand (OneOrMore)
		let seg = PathPatternSegment::new("*foo");
		seg.modifier().xpect_eq(PathPatternModifier::OneOrMore);
		seg.name().xpect_eq("foo");

		// Optional Wildcard shorthand (ZeroOrMore)
		let seg = PathPatternSegment::new("*foo?");
		seg.modifier().xpect_eq(PathPatternModifier::ZeroOrMore);
		seg.name().xpect_eq("foo");
	}

	#[test]
	fn annotated_output() {
		PathPatternSegment::new("foo")
			.to_string_annotated()
			.xpect_eq("foo".to_string());
		PathPatternSegment::new(":foo")
			.to_string_annotated()
			.xpect_eq(":foo".to_string());
		PathPatternSegment::new(":foo?")
			.to_string_annotated()
			.xpect_eq(":foo?".to_string());
		PathPatternSegment::new(":foo+")
			.to_string_annotated()
			.xpect_eq("*foo".to_string());
		PathPatternSegment::new(":foo*")
			.to_string_annotated()
			.xpect_eq("*foo?".to_string());
		PathPatternSegment::new("*foo")
			.to_string_annotated()
			.xpect_eq("*foo".to_string());
		PathPatternSegment::new("*foo?")
			.to_string_annotated()
			.xpect_eq("*foo?".to_string());
	}

	#[test]
	fn greedy_must_be_last() {
		// Greedy patterns must be last
		PathPattern::new("*foo/bar").xpect_err();
		PathPattern::new(":foo+/bar").xpect_err();
		PathPattern::new(":foo*/bar").xpect_err();

		// Non-greedy patterns can be anywhere
		PathPattern::new(":foo/bar").xpect_ok();
		PathPattern::new(":foo?/bar").xpect_ok();
	}
}
