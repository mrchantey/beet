#[cfg(feature = "tokens")]
use crate::as_beet::*;
use crate::prelude::*;
use bevy::prelude::*;
use http::request::Parts;
use std::collections::VecDeque;
use std::ops::ControlFlow;


/// A filter for matching routes based on path segments and HTTP methods.
/// This is used to determine whether a handler should be invoked for a given request,
/// and whether its children should be processed.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[reflect(Component)]
pub struct RouteFilter {
	/// Segements that must match in order for the route to be valid,
	/// an empty vector means only the root path `/` is valid.
	pub segments: Vec<RouteSegment>,
	/// Methods that this route filter applies to,
	/// an empty vector means all methods.
	/// The first method in the vector is considered the canonical method
	/// for this route, and will be used when generating route info.
	pub methods: Vec<HttpMethod>,
}


impl RouteFilter {
	pub fn new(path: &str) -> Self {
		let segments = path
			.split('/')
			.filter(|s| !s.is_empty())
			.map(RouteSegment::new)
			.collect::<Vec<_>>();
		Self {
			segments,
			methods: Vec::new(),
		}
	}
	pub fn with_method(mut self, method: HttpMethod) -> Self {
		self.methods.push(method);
		self
	}
	pub fn set_methods(mut self, methods: Vec<HttpMethod>) -> Self {
		self.methods = methods;
		self
	}

	/// Consume a segment of the path for each segment in the filter,
	/// returning the remaining path if all segments match.
	pub fn matches(
		&self,
		mut parts: RouteParts,
	) -> ControlFlow<(), RouteParts> {
		// if methods are specified, check if the method matches
		if !self.methods.is_empty() && !self.methods.contains(&parts.method) {
			return ControlFlow::Break(());
		}
		// if segments is empty, only the root path is valid
		if self.segments.is_empty() && !parts.path.is_empty() {
			return ControlFlow::Break(());
		}

		// check each segment against the path
		for segment in &self.segments {
			let next = parts.path.pop_front();
			match segment.matches(next.as_ref()) {
				ControlFlow::Break(_) => {
					return ControlFlow::Break(());
				}
				ControlFlow::Continue(()) => {}
			}
		}
		ControlFlow::Continue(parts)
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

impl RouteSegment {
	/// Parses a segment from a string, determining if it is static, dynamic, or wildcard.
	///
	/// ## Panics
	/// - Panics if the segment is empty after trimming leading and trailing slashes.
	/// - Panics if the segment contains internal slashes '/'
	pub fn new(segment: &str) -> Self {
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

	/// Attempts to match the segment against a path,
	/// returning the remaining path if it matches.
	pub fn matches(&self, segment: Option<&String>) -> ControlFlow<()> {
		match (self, segment) {
			// static match, continue with remaining path
			(RouteSegment::Static(val), Some(other)) if val == other => {
				ControlFlow::Continue(())
			}
			// dynamic will always match, continue with remaining path
			(RouteSegment::Dynamic(_), Some(_)) => ControlFlow::Continue(()),
			// wildcard consumes the rest of the path, continue with empty path
			(RouteSegment::Wildcard(_), Some(_)) => {
				ControlFlow::Continue(Default::default())
			}
			// only a wildcard permits an empty path
			(RouteSegment::Wildcard(_), None) => {
				ControlFlow::Continue(Default::default())
			}
			// break if empty path or no matching static
			(RouteSegment::Static(_) | RouteSegment::Dynamic(_), _) => {
				ControlFlow::Break(())
			}
		}
	}
}


/// A [`RoutePath`] split into segments for easier matching,
/// where each segment is guaranteed to be:
/// - non-empty
/// - not contain internal slashes `/`
#[derive(Debug, Default, Clone)]
pub struct RouteParts {
	method: HttpMethod,
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
		RouteFilter::new(filter)
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

	#[test]
	#[rustfmt::skip]
	fn method_filter() {
		fn expect_method(
			filter: Vec<HttpMethod>,
			request: HttpMethod,
		) -> Matcher<ControlFlow<(), RouteParts>> {
			RouteFilter::new("*")
				.set_methods(filter)
				.matches(RouteParts::new("", request))
				.xpect()
		}
		
		expect_method(vec![], HttpMethod::Get)
			.to_continue();
		expect_method(vec![], HttpMethod::Post)
			.to_continue();
		expect_method(vec![HttpMethod::Get], HttpMethod::Get)
			.to_continue();
		expect_method(vec![HttpMethod::Get, HttpMethod::Post], HttpMethod::Get)
			.to_continue();
		expect_method(vec![HttpMethod::Get, HttpMethod::Post], HttpMethod::Post)
			.to_continue();
		expect_method(vec![HttpMethod::Post], HttpMethod::Get)
			.to_break();
	}
}
