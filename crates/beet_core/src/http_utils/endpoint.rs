use crate::as_beet::*;
use bevy::prelude::*;

#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct Endpoint {
	method: HttpMethod,
	cache_strategy: CacheStrategy,
}

impl Endpoint {
	pub fn new(method: HttpMethod) -> Self {
		Self {
			method,
			cache_strategy: default(),
		}
	}
	pub fn new_with(method: HttpMethod, cache_type: CacheStrategy) -> Self {
		Self {
			method,
			cache_strategy: cache_type,
		}
	}
	pub fn method(&self) -> HttpMethod { self.method }
	pub fn cache_strategy(&self) -> CacheStrategy { self.cache_strategy }
}

impl Into<Endpoint> for HttpMethod {
	fn into(self) -> Endpoint { Endpoint::new(self) }
}


#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct ResolvedEndpoint {
	endpoint: Endpoint,
	path: RoutePath,
	segments: Vec<RouteSegment>,
}

impl ResolvedEndpoint {
	pub fn new(endpoint: Endpoint, segments: Vec<RouteSegment>) -> Self {
		let path = RoutePath::from(
			segments
				.iter()
				.map(|segment| segment.to_string_annotated())
				.collect::<Vec<_>>()
				.join("/"),
		);
		Self {
			endpoint,
			path,
			segments,
		}
	}
	pub fn method(&self) -> HttpMethod { self.endpoint.method }
	pub fn cache_strategy(&self) -> CacheStrategy {
		self.endpoint.cache_strategy
	}
	pub fn endpoint(&self) -> &Endpoint { &self.endpoint }

	pub fn path(&self) -> &RoutePath { &self.path }

	pub fn collect(
		query: Query<(Entity, &Endpoint)>,
		parents: Query<&ChildOf>,
		path_filters: Query<&RouteFilter>,
	) -> Vec<Self> {
		query
			.iter()
			.map(|(entity, endpoint)| {
				let mut segments = Vec::new();
				for parent in parents
					.iter_ancestors_inclusive(entity)
					.collect::<Vec<_>>()
					.into_iter()
					// reverse to start from the root
					.rev()
				{
					if let Ok(filter) = path_filters.get(parent) {
						for segment in filter.segments.iter() {
							segments.push(segment.clone());
						}
					}
				}
				Self::new(endpoint.clone(), segments)
			})
			.collect()
	}
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum CacheStrategy {
	/// An endpoint that may produce different responses for the same path and method,
	/// and should not be cached
	#[default]
	Dynamic,
	/// An endpoint that always returns the same response for a given
	/// path and method, making it suitable for ssg and caching.
	Static,
}
