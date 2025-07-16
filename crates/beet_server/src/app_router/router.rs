use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Abstract router trait that can be implemented for different web frameworks
/// like Axum, Actix, etc.
// TODO rename Router and dont reexport axum::Router
pub trait AddRoute: 'static + Clone + Send + Sync {
	/// Add a route with an async handler function
	fn add_route<Fut>(
		self,
		route_info: &RouteInfo,
		func: impl 'static + Send + Sync + Clone + Fn(Request) -> Fut,
	) -> Self
	where
		Fut: Future<Output = AppResult<Response>> + Send,
		Self: Sized;
}

// Type alias for the handler function
type Handler = Arc<
	dyn 'static
		+ Send
		+ Sync
		+ Fn(Request) -> Pin<Box<dyn Send + Future<Output = AppResult<Response>>>>,
>;

/// Mock router implementation using HashMap, similar to axum::Router
#[derive(Clone)]
pub struct BeetRouter {
	routes: HashMap<RouteInfo, Handler>,
}

impl Default for BeetRouter {
	fn default() -> Self { Self::new() }
}

impl BeetRouter {
	pub fn new() -> Self {
		Self {
			routes: HashMap::new(),
		}
	}

	/// Get a handler for a specific route info
	pub fn get_handler(&self, route_info: &RouteInfo) -> Option<&Handler> {
		self.routes.get(route_info)
	}

	pub async fn oneshot(
		&self,
		info: impl Into<RouteInfo>,
	) -> AppResult<Response> {
		let route_info = info.into();
		let handler = self.get_handler(&route_info).ok_or_else(|| {
			AppError::not_found(format!("Route not found: {}", route_info))
		})?;
		handler(route_info.into()).await
	}

	/// Get all registered routes
	pub fn routes(&self) -> impl Iterator<Item = &RouteInfo> {
		self.routes.keys()
	}

	/// For testing, collect all routes and return the base route as a string
	#[cfg(test)]
	pub async fn route_str(
		world: &mut World,
		route: impl Into<RouteInfo>,
	) -> AppResult<String> {
		world
			.run_system_cached_with(collect_routes, BeetRouter::default())
			.unwrap()?
			.oneshot(route)
			.await?
			.xmap(|res| res.body_str().unwrap())
			.xok()
	}
}

impl AddRoute for BeetRouter {
	fn add_route<Fut>(
		mut self,
		route_info: &RouteInfo,
		func: impl 'static + Send + Sync + Clone + Fn(Request) -> Fut,
	) -> Self
	where
		Fut: Future<Output = AppResult<Response>> + Send,
		Self: Sized,
	{
		let handler: Handler = Arc::new(move |request: Request| {
			let func = func.clone();
			Box::pin(async move { func(request).await })
		});

		self.routes.insert(route_info.clone(), handler);
		self
	}
}

/// Generic route collection function that works with any router implementing BeetRouter
pub fn collect_routes<R: AddRoute>(
	router: In<R>,
	workspace_config: Option<Res<WorkspaceConfig>>,
	html_constants: Option<Res<HtmlConstants>>,
	query: Query<(
		Entity,
		&RouteInfo,
		Option<&RouteScene>,
		Option<&RouteHandler>,
	)>,
	layers: Query<&RouteLayer>,
	parents: Query<&ChildOf>,
) -> Result<R> {
	let mut router = router.0;

	for (entity, route_info, route_scene, handler) in query.iter() {
		let workspace_config = workspace_config
			.as_ref()
			.map(|res| (**res).clone())
			.unwrap_or_default();
		let html_constants = html_constants
			.as_ref()
			.map(|res| (**res).clone())
			.unwrap_or_default();

		let handler = handler.cloned();
		let route_scene = route_scene.cloned();
		let layers = parents
			.iter_ancestors_inclusive(entity)
			.filter_map(|e| layers.get(e).ok().cloned())
			.collect::<Vec<_>>();

		router = router.add_route(route_info, move |request| {
			let start_time = std::time::Instant::now();

			let workspace_config = workspace_config.clone();
			let html_constants = html_constants.clone();
			let handler = handler.clone();
			let route_scene = route_scene.clone();
			let layers = layers.clone();

			async move {
				let mut world = {
					let mut app = App::new();
					app.add_plugins((AppRouterPlugin, TemplatePlugin))
						.insert_resource(workspace_config)
						.insert_resource(html_constants);

					#[cfg(all(not(test), feature = "build"))]
						app.add_plugins(
							beet_build::prelude::BuildPlugin::default(),
						);

					for layer in layers {
						layer.add_to_app(&mut app);
					}
					std::mem::take(app.world_mut())
				};

				world.insert_resource(request);

				if let Some(route_scene) = route_scene {
					world.load_scene(route_scene.ron).map_err(|err| {
						AppError::bad_request(format!(
							"Failed to load scene: {err}"
						))
					})?;
				}

				world.try_run_schedule(BeforeRoute).ok();

				if let Some(handler) = handler {
					// if handler errors it is inserted into RouteHandlerOutput
					world = handler.run(world).await
				}

				world.try_run_schedule(AfterRoute).ok();
				if !world.contains_resource::<Response>() {
					world.try_run_schedule(CollectResponse).ok();
				}

				let response =
					world.remove_resource::<Response>().unwrap_or_default();

				debug!(
					"Route handler completed in: {:.2?}",
					start_time.elapsed()
				);

				Ok(response)
			}
		});
	}

	Ok(router)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn beet_route_works() {
		let mut world = World::new();
		world.spawn(children![(
			RouteInfo::get("/"),
			RouteHandler::new(|mut commands: Commands| {
				commands.insert_resource("hello world!".into_response());
			})
		),]);

		BeetRouter::route_str(&mut world, "/")
			.await
			.unwrap()
			.xpect()
			.to_be_str("hello world!");
	}
}
