use crate::prelude::*;
use beet_core::http_resources::Request;
use beet_core::http_resources::Response;
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
		Option<&AsyncRouteHandler>,
	)>,
	layers: Query<&RouteLayer>,
	parents: Query<&ChildOf>,
) -> Result<R> {
	let mut router = router.0;

	for (entity, route_info, route_scene, handler, async_handler) in
		query.iter()
	{
		match (handler, async_handler) {
			(Some(_), Some(_)) => {
				bevybail!(
					"Route cannot have both a sync and async handler\nRoute: {:?}",
					route_info
				);
			}
			(None, None) => continue,
			_ => {
				// exactly one handler is present
			}
		};

		let workspace_config = workspace_config
			.as_ref()
			.map(|res| (**res).clone())
			.unwrap_or_default();
		let html_constants = html_constants
			.as_ref()
			.map(|res| (**res).clone())
			.unwrap_or_default();

		let handler = handler.cloned();
		let async_handler = async_handler.cloned();
		let route_scene = route_scene.cloned();
		let layers = parents
			.iter_ancestors_inclusive(entity)
			.filter_map(|e| layers.get(e).ok().cloned())
			.collect::<Vec<_>>();

		router = router.add_route(route_info, move |request| {
			let workspace_config = workspace_config.clone();
			let html_constants = html_constants.clone();
			let handler = handler.clone();
			let async_handler = async_handler.clone();
			let route_scene = route_scene.clone();
			let layers = layers.clone();

			Box::pin(async move {
				let start_time = std::time::Instant::now();

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

				world.run_schedule(Update);

				if let Some(handler) = handler {
					handler.run(&mut world)?;
				}
				if let Some(async_handler) = async_handler {
					async_handler.run(&mut world).await?;
				}

				world.run_schedule(Update);

				trace!(
					"Route handler completed in: {:.2?}",
					start_time.elapsed()
				);

				let response =
					world.remove_resource::<Response>().unwrap_or_default();

				Ok(response)
			})
		});
	}

	Ok(router)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::http_resources::IntoResponse;
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

		world
			.run_system_cached_with(collect_routes, BeetRouter::default())
			.unwrap()
			.unwrap()
			.oneshot("/")
			.await
			.unwrap()
			.xmap(|res| res.body_str().unwrap())
			.xpect()
			.to_be("hello world!".to_string());
	}
}
