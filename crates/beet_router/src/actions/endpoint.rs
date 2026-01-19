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
	/// An optional description for this endpoint
	description: Option<String>,
	params: ParamsPattern,
	/// The full [`PathPattern`] for this endpoint
	path: PathPattern,
	/// The method to match, or None for any method.
	method: Option<HttpMethod>,
	/// The cache strategy for this endpoint, if any
	cache_strategy: Option<CacheStrategy>,
	/// Marks this endpoint as an HTML endpoint
	content_type: Option<ContentType>,
	/// Canonical endpoints are registered in the EndpointTree. Non-canonical endpoints
	/// are fallbacks that won't conflict with canonical routes. Defaults to `true`.
	is_canonical: bool,
}


impl Endpoint {
	#[cfg(test)]
	pub(crate) fn new(
		path: PathPattern,
		params: ParamsPattern,
		method: Option<HttpMethod>,
		cache_strategy: Option<CacheStrategy>,
		content_type: Option<ContentType>,
		is_canonical: bool,
	) -> Self {
		Self {
			path,
			params,
			method,
			cache_strategy,
			content_type,
			is_canonical,
			description: None,
		}
	}

	pub fn description(&self) -> Option<&str> { self.description.as_deref() }
	pub fn path(&self) -> &PathPattern { &self.path }
	pub fn params(&self) -> &ParamsPattern { &self.params }
	pub fn method(&self) -> Option<HttpMethod> { self.method }
	pub fn cache_strategy(&self) -> Option<CacheStrategy> {
		self.cache_strategy
	}
	pub fn content_type(&self) -> Option<ContentType> { self.content_type }
	pub fn is_canonical(&self) -> bool { self.is_canonical }

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
	/// The action to handle the request, by default always returns a 200 OK
	insert: Box<dyn 'static + Send + Sync + FnOnce(&mut EntityWorldMut)>,
	/// The path to match, or None for any path
	path: Option<PathPartial>,
	/// The params to match, or None for any params
	params: Option<ParamsPartial>,
	/// The method to match, or None for any method. Defaults to GET
	method: Option<HttpMethod>,
	/// The cache strategy for this endpoint, if any
	cache_strategy: Option<CacheStrategy>,
	/// Specify the content type for this endpoint
	content_type: Option<ContentType>,
	/// Whether to match the path exactly, defaults to true.
	exact_path: bool,
	/// Optional description for this endpoint
	description: Option<String>,
	/// Whether this endpoint is canonical (registered in EndpointTree), defaults to true
	is_canonical: bool,
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
				entity.insert(endpoint_action(StatusCode::Ok));
			}),
			path: None,
			params: None,
			method: Some(HttpMethod::Get),
			cache_strategy: None,
			content_type: None,
			exact_path: true,
			description: None,
			is_canonical: true,
			additional_predicates: Vec::new(),
		}
	}
}

impl EndpointBuilder {
	/// Create a new endpoint builder with default settings.
	/// Use [`Self::with_handler`] to specify the action to handle the request.
	///
	/// # Example
	/// ```ignore
	/// EndpointBuilder::new()
	///     .with_path("/foo")
	///     .with_action(|| StatusCode::Ok)
	/// ```
	pub fn new() -> Self { Self::default() }

	pub fn get() -> Self { Self::default().with_method(HttpMethod::Get) }
	pub fn post() -> Self { Self::default().with_method(HttpMethod::Post) }
	pub fn any_method() -> Self { Self::default().with_any_method() }

	/// Create middleware that accepts trailing path segments and any HTTP method.
	/// Middleware runs for all matching paths and does not consume the request.
	///
	/// Unlike traditional routers, beet middleware has deep understanding of the state
	/// of the exchange, for instance it can be used for templating content, where an
	/// endpoint inserts a [`HtmlBundle`](beet_rsx::prelude::HtmlBundle), and the middleware
	/// moves it into a layout.
	///
	/// It can also be used for traditional request/response middleware, see [common_middleware](./common_middleware.rs)
	///
	/// # Example
	/// ```
	/// # use beet_router::prelude::*;
	/// # use beet_core::prelude::*;
	/// # use beet_flow::prelude::*;
	/// # use beet_net::prelude::*;
	/// // Middleware that wraps HTML content in a layout
	/// EndpointBuilder::middleware(
	///     "blog",
	///     OnSpawn::observe(|ev: On<GetOutcome>, mut commands: Commands| {
	///         // Query for HtmlBundle on agent, wrap it, trigger Outcome::Pass
	///         commands.entity(ev.target()).trigger_target(Outcome::Pass);
	///     })
	/// );
	/// ```
	pub fn middleware(
		path: impl AsRef<str>,
		handler: impl 'static + Send + Sync + Bundle,
	) -> impl Bundle {
		(
			Name::new(format!("Middleware: {}", path.as_ref())),
			Sequence,
			PathPartial::new(path.as_ref()),
			children![partial_path_match(), handler],
		)
	}
	/// Set the action to handle the request.
	///
	/// The handler is a [`BundleFunc`] that returns a bundle to be inserted as the endpoint action.
	/// For simple request/response handlers, prefer using [`Self::with_action`] which
	/// automatically wraps your handler in [`endpoint_action`]:
	///
	/// # Example
	/// ```ignore
	/// EndpointBuilder::new()
	///     .with_path("/foo")
	///     .with_action(|req: Request| req.mirror())
	/// ```
	pub fn with_handler(self, handler: impl Bundle) -> Self {
		self.with_handler_bundle(handler)
	}
	/// Convenience method that wraps the action in [`endpoint_action`].
	///
	/// This is equivalent to `.with_handler(endpoint_action(action))` and is
	/// the recommended way to add request/response handlers.
	///
	/// # Example
	/// ```ignore
	/// EndpointBuilder::new()
	///     .with_path("/foo")
	///     .with_action(|req: Request| req.mirror())
	/// ```
	pub fn with_action<M>(
		self,
		action: impl 'static + Send + Sync + Clone + IntoEndpointAction<M>,
	) -> Self {
		self.with_handler(endpoint_action(action))
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

	pub fn with_path(mut self, path: impl AsRef<str>) -> Self {
		self.path = Some(PathPartial::new(path.as_ref()));
		self
	}

	pub fn with_params<T: bevy_reflect::Typed>(mut self) -> Self {
		self.params = Some(ParamsPartial::new::<T>());
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

	/// Sets a description for this endpoint, used in help output
	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}

	/// Sets [`Self::exact_path`] to false
	pub fn with_trailing_path(mut self) -> Self {
		self.exact_path = false;
		self
	}

	/// Mark this endpoint as non-canonical, preventing it from being registered
	/// in the EndpointTree. Use this for fallback endpoints that shouldn't conflict
	/// with canonical routes.
	pub fn non_canonical(mut self) -> Self {
		self.is_canonical = false;
		self
	}

	fn effect(self, entity: &mut EntityWorldMut) {
		// the entity to eventually call [`Self::insert`] on, this will
		// be some nested entity depending on the builder configuration
		if let Some(pattern) = self.path {
			entity.insert(pattern);
		}
		if let Some(params) = self.params {
			entity.insert(params);
		}

		let id = entity.id();
		let path: PathPattern = entity.world_scope(|world| {
			world
				.run_system_cached_with(PathPattern::collect_system, id)
				.unwrap()
		});
		let params = entity
			.world_scope(|world| -> Result<ParamsPattern> {
				world
					.run_system_cached_with(ParamsPattern::collect_system, id)
					.unwrap()
			})
			.unwrap();

		entity
			.insert((
				Name::new(format!("Endpoint: {}", path.annotated_route_path())),
				Endpoint {
					path,
					params,
					description: self.description,
					method: self.method,
					cache_strategy: self.cache_strategy,
					content_type: self.content_type,
					is_canonical: self.is_canonical,
				},
				Sequence,
			))
			.with_children(|spawner| {
				// here we add the predicates as prior
				// children in the behavior tree.
				// Order is not important so long as the
				// handler is last.
				spawner.spawn(path_match(self.exact_path));

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
pub fn exact_path_match() -> impl Bundle { path_match(true) }
/// Will trigger [`Outcome::Pass`] if the request [`RoutePath`] satisfies the [`PathPattern`]
/// at this point in the tree, even if there are remaining parts.
pub fn partial_path_match() -> impl Bundle { path_match(false) }

fn path_match(must_exact_match: bool) -> impl Bundle {
	(
		Name::new("Check Path Match"),
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      mut commands: Commands,
			      query: RouteQuery| {
				let action = ev.target();
				let outcome = match query.path_match(action) {
					// expected exact match, got partial match
					Ok(path_match)
						if must_exact_match && !path_match.exact_match() =>
					{
						Outcome::Fail
					}
					// got match
					Ok(_) => Outcome::Pass,
					// match failed
					Err(_err) => Outcome::Fail,
				};
				commands.entity(action).trigger_target(outcome);
			},
		),
	)
}


fn check_method(method: HttpMethod) -> impl Bundle {
	(
		Name::new("Method Check"),
		method,
		OnSpawn::observe(
			|ev: On<GetOutcome>,
			 query: RouteQuery,
			 actions: Query<&HttpMethod>,
			 mut commands: Commands|
			 -> Result {
				let action = ev.target();
				let method = actions.get(action)?;
				let outcome = match query.method(action)? == *method {
					true => Outcome::Pass,
					false => Outcome::Fail,
				};
				commands.entity(action).trigger_target(outcome);
				Ok(())
			},
		),
	)
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn simple() {
		let _ = EndpointBuilder::new().with_action(|| {});
		let _ = EndpointBuilder::new()
			.with_action(|| -> Result<(), String> { Ok(()) });

		RouterPlugin::world()
			.spawn(flow_exchange(|| EndpointBuilder::get()))
			.exchange(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::Ok);
	}

	#[beet_core::test]
	async fn dynamic_path() {
		RouterPlugin::world()
			.spawn(flow_exchange(|| {
				EndpointBuilder::get().with_path("/:path").with_action(
					async |_req: (),
					       action: AsyncEntity|
					       -> Result<Html<String>> {
						let path =
							RouteQuery::dyn_segment_async(action, "path")
								.await?;
						Html(path).xok()
					},
				)
			}))
			.exchange_str(Request::get("/bing"))
			.await
			.xpect_eq("bing");
	}

	#[beet_core::test]
	async fn children() {
		use beet_flow::prelude::*;

		let mut world = RouterPlugin::world();
		let mut entity = world.spawn(flow_exchange(|| {
			(InfallibleSequence, children![
				EndpointBuilder::get()
					.with_path("foo")
					.with_action(|| "foo"),
				EndpointBuilder::get()
					.with_path("bar")
					.with_action(|| "bar"),
			])
		}));
		entity.exchange_str("/foo").await.xpect_eq("foo");
		entity.exchange_str("/bar").await.xpect_eq("bar");
	}

	#[beet_core::test]
	async fn works() {
		let mut world = RouterPlugin::world();
		let mut entity = world
			.spawn(flow_exchange(|| EndpointBuilder::post().with_path("foo")));

		// method and path match
		entity
			.exchange(Request::post("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::Ok);
		// method does not match - returns 500 because single endpoint failure
		// (404 requires a router with fallback structure)
		entity
			.exchange(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::InternalError);
		// path does not match
		entity
			.exchange(Request::get("/bar"))
			.await
			.status()
			.xpect_eq(StatusCode::InternalError);
		// path has extra parts
		entity
			.exchange(Request::get("/foo/bar"))
			.await
			.status()
			.xpect_eq(StatusCode::InternalError);
	}
	#[beet_core::test]
	async fn middleware_allows_trailing() {
		use beet_flow::prelude::*;

		let mut world = RouterPlugin::world();
		let mut entity = world.spawn(flow_exchange(|| {
			(InfallibleSequence, children![
				EndpointBuilder::middleware(
					"api",
					OnSpawn::observe(
						|ev: On<GetOutcome>, mut commands: Commands| {
							// Middleware just passes - demonstrates path matching
							commands
								.entity(ev.target())
								.trigger_target(Outcome::Pass);
						},
					),
				),
				EndpointBuilder::get()
					.with_path("api/users")
					.with_action(|| "users"),
			])
		}));

		// Middleware allows trailing path segments, so this matches
		entity
			.exchange(Request::get("/api/users"))
			.await
			.status()
			.xpect_eq(StatusCode::Ok);
	}


	#[test]
	fn test_collect_route_segments() {
		let mut world = World::new();
		world.spawn((
			PathPartial::new("foo"),
			EndpointBuilder::get(),
			children![
				children![
					(PathPartial::new("*bar"), EndpointBuilder::get()),
					PathPartial::new("bazz")
				],
				(PathPartial::new("qux"),),
				(PathPartial::new(":quax"), EndpointBuilder::get()),
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

	#[beet_core::test]
	async fn response_exists() {
		// Simple test to verify Response exists after endpoint
		RouterPlugin::world()
			.spawn(flow_exchange(|| {
				(InfallibleSequence, children![
					EndpointBuilder::get()
						.with_action(|| StatusCode::Ok.into_response()),
					OnSpawn::observe(
						|ev: On<GetOutcome>,
						 agents: AgentQuery,
						 response_query: Query<&Response>,
						 mut commands: Commands|
						 -> Result {
							let action = ev.target();
							let agent = agents.entity(action);
							response_query.contains(agent).xpect_true();
							commands
								.entity(action)
								.trigger_target(Outcome::Pass);
							Ok(())
						},
					),
				])
			}))
			.exchange(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::Ok);
	}
}
