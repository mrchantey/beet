use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use bevy::ecs::relationship::RelatedSpawner;


#[derive(Debug, Copy, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum ContentType {
	Html,
	Json,
}


/// Endpoints are actions that will only run if the method and path are an
/// exact match.
///
/// Usually this is not added directly, instead via the [`Endpoint::build`] constructor.
/// Endpoints should only run if there are no trailing path segments,
/// unlike middleware which may run for multiple child paths. See [`check_exact_path`]
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
pub struct Endpoint {
	/// A collection of the content of every [`PathFilter`] in this entity's
	/// ancestors(inclusive)
	route_segments: RouteSegments,
}


impl Endpoint {
	/// Call [`RouteSegments::collect`] on this entity, collecting
	/// every parent [`PathFilter`]
	pub(crate) fn new(route_segments: RouteSegments) -> Self {
		Self { route_segments }
	}

	pub fn route_segments(&self) -> &RouteSegments { &self.route_segments }
}
/// High level helper for building a correct [`Endpoint`] structure.
/// The flexibility of `beet_router` makes it challenging to build a correct
/// structure manually.
#[derive(BundleEffect)]
pub struct EndpointBuilder {
	/// The action to handle the request, by default always returns a 200 OK
	insert: Box<dyn 'static + Send + Sync + FnOnce(&mut EntityWorldMut)>,
	/// The path to match, or None for any path
	path: Option<PathFilter>,
	/// The method to match, or None for any method. Defaults to GET
	method: Option<HttpMethod>,
	/// The cache strategy for this endpoint, if any
	cache_strategy: Option<CacheStrategy>,
	/// Specify the content type for this endpoint
	content_type: Option<ContentType>,
	/// Whether to match the path exactly, defaults to true.
	exact_path: bool,
	/// Additional bundles to be run before the handler
	additional_predicates: Vec<
		Box<
			dyn 'static
				+ Send
				+ Sync
				+ FnOnce(&mut RelatedSpawner<'_, ChildOf>),
		>,
	>,
}

impl Default for EndpointBuilder {
	fn default() -> Self {
		Self {
			insert: Box::new(|entity| {
				entity.insert(StatusCode::OK.into_endpoint());
			}),
			path: None,
			method: Some(HttpMethod::Get),
			cache_strategy: None,
			content_type: None,
			exact_path: true,
			additional_predicates: Vec::new(),
		}
	}
}

impl EndpointBuilder {
	pub fn new<M>(
		handler: impl 'static + Send + Sync + IntoEndpoint<M>,
	) -> Self {
		Self::default().with_handler(handler)
	}

	pub fn get() -> Self { Self::default().with_method(HttpMethod::Get) }
	pub fn post() -> Self { Self::default().with_method(HttpMethod::Post) }
	pub fn any_method() -> Self { Self::default().with_any_method() }
	/// Create a new endpoint with the provided endpoint handler
	pub fn with_handler<M>(
		self,
		handler: impl 'static + Send + Sync + IntoEndpoint<M>,
	) -> Self {
		self.with_handler_bundle(handler.into_endpoint())
	}
	/// Create a new endpoint with the provided bundle, the bundle must be
	/// a `GetOutcome` / `Outcome` action, and usually inserts a response
	/// or some type thats later converted to a response.
	pub fn with_handler_bundle(mut self, endpoint: impl Bundle) -> Self {
		self.insert = Box::new(move |entity| {
			entity.insert(endpoint);
		});
		self
	}
	/// Create a new endpoint with the provided [`IntoMiddleware`] handler.
	/// Middleware defaults to accepting any [`HttpMethod`].
	pub fn layer<M>(
		handler: impl 'static + Send + Sync + IntoMiddleware<M>,
	) -> Self {
		Self {
			method: None,
			..default()
		}
		.with_handler_bundle(handler.into_middleware())
	}
	pub fn with_path(mut self, path: impl AsRef<str>) -> Self {
		self.path = Some(PathFilter::new(path.as_ref()));
		self
	}
	pub fn with_method(mut self, method: HttpMethod) -> Self {
		self.method = Some(method);
		self
	}
	pub fn with_any_method(mut self) -> Self {
		self.method = None;
		self
	}

	/// Add additional actions to be run before the handler,
	/// if they trigger a [`Outcome::Fail`] the handler will not run.
	pub fn with_predicate(
		mut self,
		predicate: impl Bundle + 'static + Send + Sync,
	) -> Self {
		self.additional_predicates.push(Box::new(move |spawner| {
			spawner.spawn(predicate);
		}));
		self
	}

	pub fn with_cache_strategy(mut self, strategy: CacheStrategy) -> Self {
		self.cache_strategy = Some(strategy);
		self
	}

	pub fn with_content_type(mut self, content_type: ContentType) -> Self {
		self.content_type = Some(content_type);
		self
	}

	/// Sets [`Self::exact_path`] to false
	pub fn with_trailing_path(mut self) -> Self {
		self.exact_path = false;
		self
	}

	fn effect(self, entity: &mut EntityWorldMut) {
		// the entity to eventually call [`Self::insert`] on, this will
		// be some nested entity depending on the builder configuration
		let mut current_entity = entity.id();

		if let Some(path_filter) = self.path {
			entity.insert(path_filter);
			current_entity = entity
				.world_scope(|world| world.spawn(ChildOf(current_entity)).id());
		}
		entity.world_scope(|world| {
			world
				.entity_mut(current_entity)
				.insert(Sequence)
				.with_children(|spawner| {
					// here we add the predicates as prior
					// children in the behavior tree.
					// Order is not important so long as the
					// handler is last.
					if self.exact_path {
						spawner.spawn(check_exact_path());
					}
					if let Some(method) = self.method {
						spawner.spawn(check_method(method));
					}

					for predicate in self.additional_predicates {
						(predicate)(spawner);
					}

					let mut handler_entity = spawner.spawn_empty();
					if let Some(cache_strategy) = self.cache_strategy {
						handler_entity.insert(cache_strategy);
					}
					if let Some(html) = self.content_type {
						handler_entity.insert(html);
					}
					if let Some(method) = self.method {
						handler_entity.insert(method);
					}
					(self.insert)(&mut handler_entity);
					let handler_id = handler_entity.id();
					let route_segments = spawner
						.world_mut()
						.run_system_cached_with(
							RouteSegments::collect,
							handler_id,
						)
						.unwrap();
					spawner
						.world_mut()
						.entity_mut(handler_id)
						.insert(Endpoint::new(route_segments));
				});
		});
	}
}

fn check_exact_path() -> impl Bundle {
	OnSpawn::observe(
		|mut ev: On<GetOutcome>, mut query: RouteQuery| -> Result {
			let outcome =
				query.with_cx(&mut ev, |cx| match cx.path().is_empty() {
					true => Outcome::Pass,
					false => Outcome::Fail,
				})?;
			// println!("check_exact_path: {}", outcome);
			ev.trigger_with_cx(outcome);
			Ok(())
		},
	)
}

fn check_method(method: HttpMethod) -> impl Bundle {
	(
		method,
		OnSpawn::observe(
			|mut ev: On<GetOutcome>,
			 query: RouteQuery,
			 actions: Query<&HttpMethod>|
			 -> Result {
				let method = actions.get(ev.action())?;
				let outcome = match query.method(&ev)? == *method {
					true => Outcome::Pass,
					false => Outcome::Fail,
				};
				// println!("check_method: {}", outcome);
				ev.trigger_with_cx(outcome);

				Ok(())
			},
		),
	)
}


/// Metadata collected for an endpoint
#[derive(Debug, Clone)]
pub struct EndpointMeta {
	/// The entity this metadata is for
	entity: Entity,
	/// The segments for this endpoint
	route_segments: RouteSegments,
	/// The method to match, or None for any method.
	method: Option<HttpMethod>,
	/// The cache strategy for this endpoint, if any
	cache_strategy: Option<CacheStrategy>,
	/// Marks this endpoint as an HTML endpoint
	content_type: Option<ContentType>,
}

impl EndpointMeta {
	pub fn entity(&self) -> Entity { self.entity }
	pub fn route_segments(&self) -> &RouteSegments { &self.route_segments }
	pub fn method(&self) -> Option<HttpMethod> { self.method }
	pub fn cache_strategy(&self) -> Option<CacheStrategy> {
		self.cache_strategy
	}
	pub fn content_type(&self) -> Option<ContentType> { self.content_type }


	pub fn collect(
		query: Query<(
			Entity,
			&Endpoint,
			Option<&HttpMethod>,
			Option<&CacheStrategy>,
			Option<&ContentType>,
		)>,
	) -> Vec<Self> {
		query
			.iter()
			.map(|(entity, endpoint, method, cache_strategy, html)| Self {
				entity,
				route_segments: endpoint.route_segments().clone(),
				method: method.cloned(),
				cache_strategy: cache_strategy.cloned(),
				content_type: html.cloned(),
			})
			.collect::<Vec<_>>()
	}

	/// filter the provided list of endpoint metadata
	/// by those that are static GET endpoints
	pub fn static_get(items: In<Vec<Self>>) -> Vec<Self> {
		items
			.0
			.into_iter()
			.filter(|meta| {
				meta.route_segments.is_static()
					&& meta.method.map(|m| m == HttpMethod::Get).unwrap_or(true)
					&& meta
						.cache_strategy
						.map(|s| s == CacheStrategy::Static)
						.unwrap_or(false)
			})
			.collect()
	}
	/// filter the provided list of endpoint metadata
	/// by those that are static GET endpoints with HTML
	pub fn static_get_html(items: In<Vec<Self>>) -> Vec<Self> {
		items
			.0
			.into_iter()
			.filter(|meta| {
				meta.route_segments.is_static()
					&& meta.method.map(|m| m == HttpMethod::Get).unwrap_or(true)
					&& meta
						.cache_strategy
						.map(|s| s == CacheStrategy::Static)
						.unwrap_or(false)
					&& meta.content_type == Some(ContentType::Html)
			})
			.collect()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn simple() {
		RouterPlugin::world()
			.spawn((RouteServer, EndpointBuilder::get()))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}


	#[sweet::test]
	async fn dynamic_path() {
		RouterPlugin::world()
			.spawn((
				RouteServer,
				EndpointBuilder::get().with_path("/:path").with_handler(
					async |_req: (),
					       cx: EndpointContext|
					       -> Result<Html<String>> {
						Html(cx.dyn_segment("path").await?).xok()
					},
				),
			))
			.oneshot_str(Request::get("/bing"))
			.await
			.xpect_eq("bing");
	}

	#[sweet::test]
	async fn children() {
		use beet_flow::prelude::*;

		let mut world = RouterPlugin::world();
		let mut entity =
			world.spawn((RouteServer, InfallibleSequence, children![
				EndpointBuilder::get()
					.with_path("foo")
					.with_handler(|| "foo"),
				EndpointBuilder::get()
					.with_path("bar")
					.with_handler(|| "bar"),
			]));
		entity.oneshot_str("/foo").await.xpect_eq("foo");
		entity.oneshot_str("/bar").await.xpect_eq("bar");
	}

	#[sweet::test]
	async fn works() {
		let mut world = RouterPlugin::world();
		let mut entity = world
			.spawn((RouteServer, EndpointBuilder::post().with_path("foo")));

		// method and path match
		entity
			.oneshot(Request::post("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
		// method does not match
		entity
			.oneshot(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
		// path does not match
		entity
			.oneshot(Request::get("/bar"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
		// path has extra parts
		entity
			.oneshot(Request::get("/foo/bar"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
	}
	#[test]
	#[rustfmt::skip]
	fn test_collect_route_segments() {
		let mut world = World::new();
		world.spawn((
			PathFilter::new("foo"),
			EndpointBuilder::get(),
			children![
				children![
					(
						PathFilter::new("*bar"),
						EndpointBuilder::get()
					),
					PathFilter::new("bazz")
				],
				(
					PathFilter::new("qux"),
				),
				(
					PathFilter::new(":quax"),
					EndpointBuilder::get()
				),
			],
		));
		world.run_system_cached(EndpointMeta::collect).unwrap()
    .into_iter()
    .map(|meta| meta.route_segments().annotated_route_path())
    .collect::<Vec<_>>()
		.xpect_eq(vec![
				RoutePath::new("/foo"),
				RoutePath::new("/foo/*bar"),
				RoutePath::new("/foo/:quax")
		]);
	}
}
