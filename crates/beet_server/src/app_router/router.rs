use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use std::ops::ControlFlow;


/// Mark the root entity of the router, every router app must have exactly one
/// of these.
#[derive(Component)]
pub struct RouterRoot;

/// Collection of systems for collecting and running and route handlers
/// This type serves as the intermediarybetween the main app and the route handlers.
#[derive(Clone,Resource)]
pub struct Router {
	/// An app pool constructed from the given plugin.
	app_pool: AppPool,
	plugin: ClonePluginContainer,
}

/// insert the default handlers that assist with
fn default_handlers(
	mut commands: Commands,
#[allow(unused)]
	config:Res<WorkspaceConfig>,
	query: Query<Entity, With<RouterRoot>>,
)->Result{

	let root = query.single()?;
	let mut root = commands.entity(root);
	root.with_child((
		HandlerConditions::no_response(),
		bundle_to_html_handler(),
	));
	#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
	root.with_child((
		Bucket::new(FsBucketProvider::new(config.html_dir.into_abs()),""),
		HandlerConditions::fallback(),
		bucket_handler(),
	));

	Ok(())
}

impl Router {

	/// Create a new [`Router`] with the given plugin, adding default handlers.
	pub fn new(plugin: impl ClonePlugin) -> Self {
		let plugin = ClonePluginContainer::new(plugin);
		Self::new_no_defaults(move |app:&mut App|{
			plugin.add_to_app(app);
			app.add_systems(Startup, default_handlers);
		})
	}
	/// Create a new [`Router`] with the given plugin, default handlers are not added.
	pub fn new_no_defaults(plugin: impl ClonePlugin) -> Self {
		Self {
			plugin: ClonePluginContainer(plugin.box_clone()),
			app_pool: Self::create_app_pool(plugin),
		}
	}
/// Convenience method to create a new [`Router`] with a bundle of routes, 
/// adding the [`RouterRoot`] component.
	pub fn new_bundle<B>(func: impl 'static + Send + Sync + Clone + FnOnce() -> B) -> Self 
		where B: Bundle{
		Self::new(move |app:&mut App|{
			app.world_mut().spawn((
				RouterRoot,
				func.clone()(),
			));
		})
	}


	/// Creates a new [`Plugin`] which will first add the [`Self::plugin`]
	/// and then the new plugin.
	pub fn add_plugin(&mut self, plugin: impl ClonePlugin) {
		let current_plugin = self.plugin.clone();
		let new_plugin = ClonePluginContainer::new(plugin);
		let plugin = move |app:&mut App|{
			current_plugin.add_to_app(app);
			new_plugin.add_to_app(app);
		};
		self.plugin = ClonePluginContainer(plugin.box_clone());
		self.app_pool = Self::create_app_pool(plugin);
	}
	
	fn create_app_pool(plugin: impl ClonePlugin) -> AppPool {
		AppPool::new(move || {
			let mut app = App::new();
			app.add_plugins((
				RouterPlugin,
				#[cfg(not(test))]
				LoadSnippetsPlugin,
			));
			plugin.add_to_app(&mut app);
			app.init();
			app.update();
			app
		})
	}

	pub fn world(&self) -> PooledWorld {
		self.app_pool.pop()
	}

	pub fn from_world(&self,world:&mut World)->Result<Self>{

		let render_mode = world.resource::<RenderMode>().clone();
		let mut this = self.clone();
		this.add_plugin(move |app: &mut App| {
			app.insert_resource(render_mode.clone());
			},
		);
		let mut world = this.app_pool.pop();
		let num_roots = world
			.query_filtered_once::<(), With<RouterRoot>>()
			.len();
		if num_roots != 1 {
			bevybail!(
				"Router apps must have exactly one `RouterRoot`, found {num_roots}",
			);
		}

		Ok(this)
	}


	/// Handle a single request, returning the response or a 404 if not found.
	pub async fn oneshot(&self, req: impl Into<Request>) -> Response {
		self.handle_request(req.into()).await
	}
	pub async fn oneshot_str(&self, req: impl Into<Request>) -> Result<String> {
		let res = self.oneshot(req).await.into_result().await?;
		res.text().await
	}


	/// Handle a request in the world, returning the response
	pub async fn handle_request(&self, request: Request) -> Response {
		// let mut world = self.app_pool.pop();
		// TODO proper pooling, this creates new app each time
		let mut world = self.app_pool.pop();

		let start_time = CrossInstant::now();

		let route_parts = RouteParts::from_parts(&request.parts);

		trace!("Handling request: {:#?}", request);
		world.insert_resource(request);

		let root = world.query_filtered::<Entity, With<RouterRoot>>().single(&world).expect(
			"Router apps must have exactly one `RouterRoot`",
		);
		// let mut owned_world = std::mem::take(world);
		world = self
			.handle_request_recursive(world, route_parts.clone(), root)
			.await;

		let response =world.remove_resource::<Response>().unwrap_or_else(|| {
			Response::not_found()
		});

		trace!("Returning Response: {:#?}", response);
		trace!("Route handler completed in: {:.2?}", start_time.elapsed());

		response
	}
	/// Pre-order depth fist traversal. parent first, then children.
	async fn handle_request_recursive(
		&self,
		mut world: PooledWorld,
		parts: RouteParts,
		root_entity: Entity,
	) -> PooledWorld {
		struct StackFrame {
			entity: Entity,
			parts: RouteParts,
		}

		let mut stack = vec![StackFrame {
			entity: root_entity,
			parts,
		}];

		while let Some(StackFrame { entity, mut parts }) = stack.pop() {
			// Check 1: MethodFilter
			if let Some(method_filter) =
				world.entity(entity).get::<MethodFilter>()
			{
				if !method_filter.matches(&parts) {
					// method does not match, skip this entity
					continue;
				}
			}
			// Check 2: PathFilter
			if let Some(filter) = world.entity(entity).get::<PathFilter>() {
				match filter.matches(parts.clone()) {
					ControlFlow::Break(_) => {
						// path does not match, skip this entity
						continue;
					}
					ControlFlow::Continue(remaining_parts) => {
						parts = remaining_parts;
					}
				}
			}

			// at this point add children, even if the endpoint doesnt match
			// a child might
			if let Some(children) = world.entity(entity).get::<Children>() {
				// reverse children to maintain order with stack.pop()
				for child in children.iter().rev() {
					stack.push(StackFrame {
						entity: child,
						parts: parts.clone(),
					});
				}
			}

			// Check 3: Endpoint
			if let Some(endpoint) = world.entity(entity).get::<Endpoint>() {
				if
				// endpoints may only run if exact path match
				!parts.path().is_empty() || 
				// method must match
				endpoint.method() != parts.method()
				{
					continue;
				}
			}


			// Check 4: HandlerPredicates
			if let Some(predicates) =
				world.entity(entity).get::<HandlerConditions>().cloned()
			{
				let (world2, should_run) =
					predicates.should_run(std::mem::take(world.inner_mut()), entity).await;
				*world.inner_mut() = world2;
				if !should_run {
					continue;
				}
			}

			// Party time: actually run the handler
			if let Some(handler) =
				world.entity(entity).get::<RouteHandler>().cloned()
			{
				*world.inner_mut() = handler
					.clone()
					.run(std::mem::take(world.inner_mut()),entity)
					.await;
			}
		}
		world
	}
}




/// insert a route tree for the current world, added at startup by the [`RouterPlugin`].
pub fn insert_route_tree(world: &mut World) {
	let endpoints = world.run_system_cached(ResolvedEndpoint::collect).unwrap();
	let paths = endpoints
		.into_iter()
		.map(|(entity, endpoint)| (entity, endpoint.path().clone()))
		.collect();
	world.insert_resource(RoutePathTree::from_paths(paths));
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[derive(Default, Resource, Deref, DerefMut)]
	struct Foo(Vec<u32>);

	#[sweet::test]
	async fn beet_route_works() {
		let router = Router::new_bundle(|| {
				RouteHandler::new(HttpMethod::Get, || "hello world!")
		});
		router
			.oneshot_str("/")
			.await
			.unwrap()
			.xpect()
			.to_be_str("hello world!");
	}


	async fn parse(route: &str) -> Vec<u32> {
		let router = Router::new(|app: &mut App| {
			app.world_mut().spawn((
				// PathFilter::new("/"),
				RouterRoot,				
				RouteHandler::layer(|mut res: ResMut<Foo>| {
					res.push(0);
				}),
				children![
				(
					PathFilter::new("foo"),
					RouteHandler::layer(|mut res: ResMut<Foo>| {
						res.push(1);
					}),
					children![
						(
							PathFilter::new("bar"),
							Endpoint::new(HttpMethod::Get),
							RouteHandler::layer(|mut res: ResMut<Foo>| {
								res.push(2);
							}),
						),
						(
							PathFilter::new("bazz"),
							Endpoint::new(HttpMethod::Delete),
							RouteHandler::layer(|mut res: ResMut<Foo>| {
								res.push(3);
							}),
						),
						(
							// no endpoint, always runs if parent matches
							RouteHandler::layer(|mut res: ResMut<Foo>| {
								res.push(4);
							}),
						),
					],
				),
				(RouteHandler::layer(
					|mut commands: Commands, res: ResMut<Foo>| {
						commands.insert_resource(
							Json(res.0.clone()).into_response(),
						);
					}
				),)
			]));

			app.world_mut().init_resource::<Foo>();
		});
		router
			.handle_request(Request::get(route))
			.await
			.json()
			.await
			.unwrap()
	}

	#[sweet::test]
	async fn works() {
		parse("/").await.xpect().to_be(vec![0]);
		parse("/foo").await.xpect().to_be(vec![0, 1, 4]);
		parse("/foo/chicken").await.xpect().to_be(vec![0, 1, 4]);
		parse("/foo/bar").await.xpect().to_be(vec![0, 1, 2, 4]);
		// path matches, method does not
		parse("/foo/bazz").await.xpect().to_be(vec![0, 1, 4]);
	}
	#[sweet::test]
	async fn simple() {
		let router = Router::new_bundle(|| {
			(
				PathFilter::new("pizza"),
				RouteHandler::new(HttpMethod::Get, || "hawaiian"),
			)
		});
		router
			.oneshot_str("/sdjhkfds")
			.await
			.unwrap_err()
			.to_string()
			.xpect()
			.to_be("404 Not Found\n");
		router
			.oneshot_str("/pizza")
			.await
			.unwrap()
			.xpect()
			.to_be_str("hawaiian");
	}
	#[sweet::test]
	async fn endpoint_with_children() {
		let router = Router::new_bundle(|| {
			(
				PathFilter::new("foo"),
				RouteHandler::new(HttpMethod::Get, || "foo"),
				children![(
					PathFilter::new("bar"),
					RouteHandler::new(HttpMethod::Get, || "bar")
				),
				],
			)
		});
		router
			.oneshot_str("/foo")
			.await
			.unwrap()
			.xpect()
			.to_be_str("foo");
		router
			.oneshot_str("/foo/bar")
			.await
			.unwrap()
			.xpect()
			.to_be_str("bar");
	}
	#[sweet::test]
	async fn route_tree() {
		let router = Router::new(|app: &mut App| {
			app.world_mut().spawn((
				RouterRoot,
				RouteHandler::new(
					HttpMethod::Get,
					|tree: Res<RoutePathTree>| tree.to_string(),
				),
				children![
					(
						PathFilter::new("foo"),
						RouteHandler::new(HttpMethod::Get, || "foo")
					),
					(PathFilter::new("bar"), children![(
						PathFilter::new("baz"),
						RouteHandler::new(HttpMethod::Get, || "baz")
					)]),
					(PathFilter::new("boo"),),
				],
			));
			app.world_mut()
				.run_system_cached(insert_route_tree)
				.unwrap();
		});
		router
			.oneshot_str("/")
			.await
			.unwrap()
			.xpect()
			.to_be_snapshot();
	}
}
