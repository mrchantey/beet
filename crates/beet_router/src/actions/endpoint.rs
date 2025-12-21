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
/// exact match. There should only be one of these per route match,
/// unlike non-endpoint entities that behave as middleware.
///
/// Usually this is not added directly, instead via the [`Endpoint::build`] constructor.
/// Endpoints should only run if there are no trailing path segments,
/// unlike middleware which may run for multiple child paths. See [`check_exact_path`]
#[derive(Debug, Clone, Component, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct Endpoint {
	params: ParamsPattern,
	/// The full [`PathPattern`] for this endpoint
	path: PathPattern,
	/// The method to match, or None for any method.
	method: Option<HttpMethod>,
	/// The cache strategy for this endpoint, if any
	cache_strategy: Option<CacheStrategy>,
	/// Marks this endpoint as an HTML endpoint
	content_type: Option<ContentType>,
}


impl Endpoint {
	pub fn path(&self) -> &PathPattern { &self.path }
	pub fn method(&self) -> Option<HttpMethod> { self.method }
	pub fn cache_strategy(&self) -> Option<CacheStrategy> {
		self.cache_strategy
	}
	pub fn content_type(&self) -> Option<ContentType> { self.content_type }

	/// Determines if this endpoint is a static GET endpoint
	pub fn is_static_get(&self) -> bool {
		self.path.is_static()
			&& self.method.map(|m| m == HttpMethod::Get).unwrap_or(true)
			&& self
				.cache_strategy
				.map(|s| s == CacheStrategy::Static)
				.unwrap_or(false)
	}
	/// Determines if this endpoint is a static GET endpoint returning HTML
	pub fn is_static_get_html(&self) -> bool {
		self.is_static_get() && self.content_type == Some(ContentType::Html)
	}
}

/// High level helper for building a correct [`Endpoint`] structure.
/// The flexibility of `beet_router` makes it challenging to build a correct
/// structure manually.
#[derive(BundleEffect)]
pub struct EndpointBuilder {
	// params: RoutePar
	/// The action to handle the request, by default always returns a 200 OK
	insert: Box<dyn 'static + Send + Sync + FnOnce(&mut EntityWorldMut)>,
	/// The path to match, or None for any path
	path: Option<PathPartial>,
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
		self.path = Some(PathPartial::new(path.as_ref()));
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
		if let Some(pattern) = self.path {
			entity.insert(pattern);
		}
		let id = entity.id();
		let path: PathPattern = entity.world_scope(|world| {
			world
				.run_system_cached_with(PathPattern::collect_system, id)
				.unwrap()
		});
		let params: ParamsPattern = entity.world_scope(|world| {
			world
				.run_system_cached_with(ParamsPattern::collect_system, id)
				.unwrap()
		});

		entity
			.insert((
				Name::new(format!("Endpoint: {}", path.annotated_route_path())),
				Endpoint {
					path,
					params,
					method: self.method,
					cache_strategy: self.cache_strategy,
					content_type: self.content_type,
				},
				Sequence,
			))
			.with_children(|spawner| {
				// here we add the predicates as prior
				// children in the behavior tree.
				// Order is not important so long as the
				// handler is last.
				spawner.spawn(route_match(self.exact_path));

				if let Some(method) = self.method {
					spawner.spawn(check_method(method));
				}

				for predicate in self.additional_predicates {
					(predicate)(spawner);
				}

				let mut handler_entity =
					spawner.spawn(Name::new("Route Handler"));
				if let Some(cache_strategy) = self.cache_strategy {
					handler_entity.insert(cache_strategy);
				}
				if let Some(content_type) = self.content_type {
					handler_entity.insert(content_type);
				}
				if let Some(method) = self.method {
					handler_entity.insert(method);
				}
				(self.insert)(&mut handler_entity);
			});
	}
}

/// Will trigger [`Outcome::Pass`] if the request [`RoutePath`] satisfies the [`PathPattern`]
/// at this point in the tree with no remaining parts.
pub fn exact_route_match() -> impl Bundle { route_match(true) }
/// Will trigger [`Outcome::Pass`] if the request [`RoutePath`] satisfies the [`PathPattern`]
/// at this point in the tree, even if there are remaining parts.
pub fn partial_route_match() -> impl Bundle { route_match(false) }

fn route_match(exact_match: bool) -> impl Bundle {
	(
		Name::new("Route Match"),
		OnSpawn::observe(
			move |mut ev: On<GetOutcome>, query: RouteQuery| -> Result {
				let outcome = match query.route_match(&ev) {
					// expected exact match, got partial match
					Ok(route_match)
						if exact_match && !route_match.exact_match() =>
					{
						Outcome::Fail
					}
					// got match
					Ok(_) => Outcome::Pass,
					// match failed
					Err(_err) => Outcome::Fail,
				};
				ev.trigger_with_cx(outcome);
				Ok(())
			},
		),
	)
}


fn check_method(method: HttpMethod) -> impl Bundle {
	(
		Name::new("Method Check"),
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



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn simple() {
		RouterPlugin::world()
			.spawn((Router, EndpointBuilder::get()))
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}


	#[sweet::test]
	async fn dynamic_path() {
		RouterPlugin::world()
			.spawn((
				Router,
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
		let mut entity = world.spawn((Router, InfallibleSequence, children![
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
		let mut entity =
			world.spawn((Router, EndpointBuilder::post().with_path("foo")));

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
			PathPartial::new("foo"),
			EndpointBuilder::get(),
			children![
				children![
					(
						PathPartial::new("*bar"),
						EndpointBuilder::get()
					),
					PathPartial::new("bazz")
				],
				(
					PathPartial::new("qux"),
				),
				(
					PathPartial::new(":quax"),
					EndpointBuilder::get()
				),
			],
		));
		let mut paths = world
			.query_once::<&Endpoint>()
			.into_iter()
			.map(|endpoint| endpoint.path().annotated_route_path())
			.collect::<Vec<_>>();
		paths.sort();
		paths.xpect_eq(vec![
			RoutePath::new("/foo"),
			RoutePath::new("/foo/*bar"),
			RoutePath::new("/foo/:quax"),
		]);
	}
}
