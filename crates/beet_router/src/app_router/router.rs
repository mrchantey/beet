use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;
use bevy::prelude::*;
use std::collections::VecDeque;
use std::ops::ControlFlow;


/// Mark the root entity of the router, every router app must have exactly one
/// of these.
#[derive(Component)]
pub struct RouterRoot;


/// Plugin added to the [`AppPool`] app for each handler
pub struct RouterAppPlugin;

impl Plugin for RouterAppPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ApplyDirectivesPlugin)
			.init_resource::<WorkspaceConfig>()
			.init_resource::<RenderMode>()
			.init_resource::<DynSegmentMap>()
			.init_resource::<HtmlConstants>()
			.add_systems(
				Startup,
				(default_handlers, insert_route_tree).chain(),
			);
	}
}



/// Collection of systems for collecting and running and route handlers
/// This type serves as the intermediary between the main app and the route handlers.
#[derive(Clone, Resource)]
pub struct Router {
	/// An app pool constructed from the given plugin.
	app_pool: AppPool,
	plugin: ClonePluginContainer,
}


impl Router {
	/// Create a new [`Router`] with the given plugin and the [`RouterAppPlugin`].
	pub fn new(plugin: impl ClonePlugin) -> Self {
		let plugin = ClonePluginContainer::new(plugin);
		Self::new_no_defaults(move |app: &mut App| {
			plugin.add_to_app(app);
			app.init_plugin(RouterAppPlugin);
			#[cfg(not(test))]
			app.add_plugins(LoadSnippetsPlugin);
		})
	}
	/// Create a new [`Router`] with the given plugin, without the [`RouterAppPlugin`].
	pub fn new_no_defaults(plugin: impl ClonePlugin) -> Self {
		Self {
			plugin: ClonePluginContainer(plugin.box_clone()),
			app_pool: Self::create_app_pool(plugin),
		}
	}
	/// Convenience method to create a new [`Router`] with a bundle of routes,
	/// adding the [`RouterRoot`] component.
	pub fn new_bundle<B>(
		func: impl 'static + Send + Sync + Clone + FnOnce() -> B,
	) -> Self
	where
		B: Bundle,
	{
		Self::new(move |app: &mut App| {
			app.world_mut().spawn((RouterRoot, func.clone()()));
		})
	}
	pub fn with_plugin(mut self, plugin: impl ClonePlugin) -> Self {
		self.add_plugin(plugin);
		self
	}

	/// Creates a new [`Plugin`] which will first add the [`Self::plugin`]
	/// and then the new plugin.
	pub fn add_plugin(&mut self, new_plugin: impl ClonePlugin) {
		let current_plugin = self.plugin.clone();
		let new_plugin = ClonePluginContainer::new(new_plugin);
		let plugin = move |app: &mut App| {
			current_plugin.add_to_app(app);
			new_plugin.add_to_app(app);
		};
		self.plugin = ClonePluginContainer(plugin.box_clone());
		self.app_pool = Self::create_app_pool(plugin);
	}

	fn create_app_pool(plugin: impl ClonePlugin) -> AppPool {
		AppPool::new(move || {
			let mut app = App::new();
			plugin.add_to_app(&mut app);
			app.init();
			app.update();
			app
		})
	}



	/// Copy some types from the parent world to the router world.
	pub fn clone_parent_world(world: &mut World) -> Result {
		let mut router = world
			.remove_resource::<Router>()
			.ok_or_else(|| bevyhow!("No Router resource found"))?;
		let render_mode = world.resource::<RenderMode>().clone();
		let package_config = world.resource::<PackageConfig>().clone();
		router.add_plugin(move |app: &mut App| {
			app.insert_resource(render_mode.clone());
			app.insert_resource(package_config.clone());
		});
		router.validate()?;
		world.insert_resource(router);
		Ok(())
	}



	/// Pop an app from the pool and run async actions on it.
	pub async fn construct_world(&self) -> PooledWorld {
		let mut world = self.app_pool.pop();
		let world2 = std::mem::take(world.inner_mut());
		*world.inner_mut() = AsyncActionSet::collect_and_run(world2).await;
		world
	}

	// check the router world has required components
	pub fn validate(&self) -> Result {
		let mut router_world = self.app_pool.pop();
		let num_roots = router_world
			.query_filtered_once::<(), With<RouterRoot>>()
			.len();
		if num_roots != 1 {
			bevybail!(
				"Router apps must have exactly one `RouterRoot`, found {num_roots}",
			);
		}
		Ok(())
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
		// TODO proper pooling, this creates new app each time
		let mut world = self.construct_world().await;

		let start_time = Instant::now();

		let route_parts = route_path_queue(&request.parts.uri.path());
		let method = request.method();

		trace!("Handling request: {:#?}", request);
		world.insert_resource(request);


		let root = world
			.query_filtered::<Entity, With<RouterRoot>>()
			.single(&world)
			.expect("Router apps must have exactly one `RouterRoot`");
		// let mut owned_world = std::mem::take(world);
		world = self
			.handle_request_recursive(world, method, route_parts, root)
			.await;

		let response = world
			.remove_resource::<Response>()
			.unwrap_or_else(|| Response::not_found());

		trace!("Returning Response: {:#?}", response);
		trace!("Route handler completed in: {:.2?}", start_time.elapsed());

		response
	}
	/// Pre-order depth fist traversal. parent first, then children.
	async fn handle_request_recursive(
		&self,
		mut world: PooledWorld,
		req_method: HttpMethod,
		req_path: VecDeque<String>,
		root_entity: Entity,
	) -> PooledWorld {
		struct StackFrame {
			entity: Entity,
			current_path: VecDeque<String>,
		}

		let mut stack = vec![StackFrame {
			entity: root_entity,
			current_path: req_path,
		}];

		while let Some(StackFrame {
			entity,
			mut current_path,
		}) = stack.pop()
		{
			let mut dyn_map =
				world.remove_resource::<DynSegmentMap>().unwrap_or_default();

			// Check 2: PathFilter
			if let Some(filter) = world.entity(entity).get::<PathFilter>() {
				match filter.matches(&mut dyn_map, &mut current_path) {
					ControlFlow::Break(_) => {
						// path does not match, skip this entity
						continue;
					}
					ControlFlow::Continue(()) => {}
				}
			}
			world.insert_resource(dyn_map);

			// at this point add children, even if the endpoint doesnt match
			// a child might
			if let Some(children) = world.entity(entity).get::<Children>() {
				// reverse children to maintain order with stack.pop()
				for child in children.iter().rev() {
					stack.push(StackFrame {
						entity: child,
						current_path: current_path.clone(),
					});
				}
			}

			// Check 3: Method and Path
			if !current_path.is_empty()
				&& world.entity(entity).contains::<Endpoint>()
			{
				continue;
			}
			if let Some(method) = world.entity(entity).get::<HttpMethod>()
				// method must match if specified
				&& *method != req_method
			{
				continue;
			}

			// Check 4: HandlerPredicates
			if let Some(predicates) =
				world.entity(entity).get::<HandlerConditions>().cloned()
			{
				let (world2, should_run) = predicates
					.should_run(std::mem::take(world.inner_mut()), entity)
					.await;
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
					.run(std::mem::take(world.inner_mut()), entity)
					.await;
			}
		}
		world
	}
}

/// insert a route tree for the current world, added at startup by the [`RouterPlugin`].
pub fn insert_route_tree(world: &mut World) {
	let paths = world.run_system_cached(static_get_routes).unwrap();
	world.insert_resource(RoutePathTree::from_paths(paths));
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[derive(Default, Resource, Deref, DerefMut)]
	struct Foo(Vec<u32>);

	#[sweet::test]
	async fn works() {
		Router::new_bundle(|| RouteHandler::endpoint(|| "hello world!"))
			.oneshot_str("/")
			.await
			.unwrap()
			.xpect_str("hello world!");
	}
	#[sweet::test]
	async fn dynamic_path() {
		Router::new_bundle(|| {
			(
				PathFilter::new("/foo/:bar"),
				RouteHandler::endpoint(|| "hello world!"),
			)
		})
		.oneshot_str("/foo/bazz")
		.await
		.unwrap()
		.xpect_str("hello world!");
	}

	#[sweet::test]
	async fn app_info() {
		Router::new(|app: &mut App| {
			app.insert_resource(RenderMode::Ssr)
				.insert_resource(pkg_config!())
				.world_mut()
				.spawn(RouterRoot);
		})
		.oneshot_str("/app-info")
		.await
		.unwrap()
		.xpect_contains("<h1>App Info</h1><p>Title: beet_router</p>");
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
								RouteHandler::layer(|mut res: ResMut<Foo>| {
									res.push(2);
								}),
							),
							(
								PathFilter::new("bazz"),
								HttpMethod::Delete,
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
					RouteHandler::layer(
						|mut commands: Commands, res: ResMut<Foo>| {
							commands.insert_resource(
								Json(res.0.clone()).into_response(),
							);
						}
					)
				],
			));

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
	async fn tree_order() {
		parse("/").await.xpect_eq(vec![0]);
		parse("/foo").await.xpect_eq(vec![0, 1, 4]);
		parse("/foo/chicken").await.xpect_eq(vec![0, 1, 4]);
		parse("/foo/bar").await.xpect_eq(vec![0, 1, 2, 4]);
		// path matches, method does not
		parse("/foo/bazz").await.xpect_eq(vec![0, 1, 4]);
	}
	#[sweet::test]
	async fn simple() {
		let router = Router::new_bundle(|| {
			(
				PathFilter::new("pizza"),
				RouteHandler::endpoint(|| "hawaiian"),
			)
		});
		router
			.oneshot_str("/sdjhkfds")
			.await
			.unwrap_err()
			.to_string()
			.xpect_eq("404 Not Found\n");
		router
			.oneshot_str("/pizza")
			.await
			.unwrap()
			.xpect_str("hawaiian");
	}
	#[sweet::test]
	async fn dynamic() {
		Router::new_bundle(|| {
			(PathFilter::new("foo"), children![(
				PathFilter::new(":bar"),
				RouteHandler::endpoint(|paths: Res<DynSegmentMap>| {
					format!("path is {}", paths.get("bar").unwrap())
				})
			),])
		})
		.oneshot_str("/foo/bazz")
		.await
		.unwrap()
		.xpect_str("path is bazz");
	}

	#[sweet::test]
	async fn endpoint_with_children() {
		let router = Router::new_bundle(|| {
			(
				PathFilter::new("foo"),
				RouteHandler::endpoint(|| "foo"),
				children![(
					PathFilter::new("bar"),
					RouteHandler::endpoint(|| "bar")
				),],
			)
		});
		router.oneshot_str("/foo").await.unwrap().xpect_str("foo");
		router
			.oneshot_str("/foo/bar")
			.await
			.unwrap()
			.xpect_str("bar");
	}
	#[sweet::test]
	async fn route_tree() {
		let router = Router::new(|app: &mut App| {
			app.world_mut().spawn((
				RouterRoot,
				CacheStrategy::Static,
				RouteHandler::endpoint(|tree: Res<RoutePathTree>| {
					tree.to_string()
				}),
				children![
					(
						PathFilter::new("foo"),
						CacheStrategy::Static,
						RouteHandler::endpoint(|| "foo")
					),
					(PathFilter::new("bar"), children![(
						PathFilter::new("baz"),
						CacheStrategy::Static,
						RouteHandler::endpoint(|| "baz")
					)]),
					(PathFilter::new("boo"),),
				],
			));
			app.world_mut()
				.run_system_cached(insert_route_tree)
				.unwrap();
		});
		router.oneshot_str("/").await.unwrap().xpect_snapshot();
	}
}
