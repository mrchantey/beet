use crate::prelude::*;
use axum::routing;
use beet_core::http_resources::Request;
use beet_core::http_resources::Response;
use bevy::prelude::*;


/// for each [`RouteInfo`] with either a [`RouteHandler`] or [`AsyncRouteHandler`],
/// collect the route into the [`Router`].
pub fn collect_axum_routes<S: DerivedAppState>(
	mut router: Router<S>,
	world: &mut World,
) -> Result<Router<S>> {
	for (route_info, route_scene, handler, async_handler) in world
		.query_once::<(
			&RouteInfo,
			Option<&RouteScene>,
			Option<&RouteHandler>,
			Option<&AsyncRouteHandler>,
		)>() {
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

		let handler = handler.cloned();
		let async_handler = async_handler.cloned();
		let route_scene = route_scene.cloned();

		router = router.route(
			&route_info.path.to_string_lossy(),
			routing::on(
				route_info.method.into_axum_method(),
				async move |state: axum::extract::State<S>,
				            request: axum::extract::Request|
				            -> AppResult<axum::response::Response> {
					let start_time = std::time::Instant::now();

					let request = Request::from_axum(request, &())
						.await
						.map_err(|err| {
							AppError::bad_request(format!(
								"Failed to extract request: {}",
								err
							))
						})?;

					let mut world =
						std::mem::take(state.create_app().world_mut());
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

					world
						.remove_resource::<Response>()
						.unwrap_or_default()
						.into_axum()
						.xok()
				},
			),
		);
	}

	Ok(router)
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::http_resources::Response;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let mut world = World::new();
		world.spawn(children![(
			RouteInfo::get("/"),
			RouteHandler::new(|mut commands: Commands| {
				commands.insert_resource(Response::new_str("hello world!"));
			})
		),]);

		let mut router = collect_axum_routes(
			Router::<AppRouterState>::default(),
			&mut world,
		)
		.unwrap()
		.with_state::<()>(AppRouterState::default());
		router
			.oneshot_str("/")
			.await
			.unwrap()
			.xpect()
			.to_be("hello world!".to_string());
	}
}
