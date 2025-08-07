use crate::as_beet::*;
use bevy::prelude::*;

#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[cfg_attr(feature = "tokens", to_tokens(Endpoint::new_with))]
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


#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Default, Component)]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct ResolvedEndpoint {
	endpoint: Endpoint,
	path: RoutePath,
	segments: Vec<RouteSegment>,
}

impl ResolvedEndpoint {
	pub fn new(
		endpoint: impl Into<Endpoint>,
		segments: Vec<RouteSegment>,
	) -> Self {
		let path = RoutePath::from(
			segments
				.iter()
				.map(|segment| segment.to_string_annotated())
				.collect::<Vec<_>>()
				.join("/"),
		);
		Self {
			endpoint: endpoint.into(),
			path,
			segments,
		}
	}
	pub fn method(&self) -> HttpMethod { self.endpoint.method }
	pub fn cache_strategy(&self) -> CacheStrategy {
		self.endpoint.cache_strategy
	}
	pub fn endpoint(&self) -> &Endpoint { &self.endpoint }
	pub fn segments(&self) -> &Vec<RouteSegment> { &self.segments }
	pub fn path(&self) -> &RoutePath { &self.path }

	pub fn collect(
		query: Query<(Entity, &Endpoint)>,
		parents: Query<&ChildOf>,
		path_filters: Query<&RouteFilter>,
	) -> Vec<(Entity, Self)> {
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
				(entity, Self::new(endpoint.clone(), segments))
			})
			.collect()
	}

	/// Collect all static GET endpoints from the world,
	/// used for differentiating ssg paths
	pub fn collect_static_get(world: &mut World) -> Vec<(Entity, Self)> {
		world
			.run_system_cached(Self::collect)
			.unwrap()
			.into_iter()
			.filter(|(_, info)| {
				info.method() == HttpMethod::Get
					&& info.cache_strategy() == CacheStrategy::Static
			})
			.collect()
	}
	/// If the endpoint is a static HTML endpoint
	// TODO check the content type, maybe store on the endpoint?
	pub fn is_static_html(&self) -> bool {
		self.method() == HttpMethod::Get
			&& self.cache_strategy() == CacheStrategy::Static
	}
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;



	#[test]
	#[rustfmt::skip]
	fn collect() {
		let mut world = World::new();
		world.spawn((
			RouteFilter::new("foo"),
			Endpoint::new(HttpMethod::Get),
			children![
				children![
					(
						RouteFilter::new("*bar"), 
						Endpoint::new(HttpMethod::Post)
					),
					RouteFilter::new("bazz")
				],
				(
					RouteFilter::new("qux"),
				),
				(
					RouteFilter::new(":quax"), 
					Endpoint::new(HttpMethod::Post)
				),
			],
		));
		world.run_system_cached(ResolvedEndpoint::collect).unwrap()
    .into_iter()
    .map(|(_, info)| info)
    .collect::<Vec<_>>()
		.xpect().to_be(vec![
			ResolvedEndpoint::new(
				HttpMethod::Get,
				vec![
					RouteSegment::Static("foo".into()),
				],
			),
			ResolvedEndpoint::new(
				HttpMethod::Post,
				vec![
					RouteSegment::Static("foo".into()),
					RouteSegment::Wildcard("bar".into()),
				],
			),
			ResolvedEndpoint::new(
				HttpMethod::Post,
				vec![
					RouteSegment::Static("foo".into()),
					RouteSegment::Dynamic("quax".into()),
				],
			),
		]);
	}
}
