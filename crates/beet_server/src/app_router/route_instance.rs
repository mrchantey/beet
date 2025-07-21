use crate::prelude::*;
use beet_core::prelude::*;
use beet_rsx::prelude::*;
use bevy::prelude::*;

/// An entirely isolated instance of a route, which may be
/// embedded in routers like axum
#[derive(Clone)]
pub struct RouteInstance {
	pub workspace_config: WorkspaceConfig,
	pub html_constants: HtmlConstants,
	pub route_info: RouteInfo,
	pub handler: Option<RouteHandler>,
	pub route_scene: Option<RouteScene>,
	pub layers: Vec<RouteLayer>,
	pub template_flags: Option<TemplateFlags>,
}


impl RouteInstance {
	pub fn from_entity(
		entity: In<Entity>,
		workspace_config: Option<Res<WorkspaceConfig>>,
		html_constants: Option<Res<HtmlConstants>>,
		template_flags: Option<Res<TemplateFlags>>,
		layers: Query<&RouteLayer>,
		parents: Query<&ChildOf>,
		query: Query<(
			Entity,
			&RouteInfo,
			Option<&RouteScene>,
			Option<&RouteHandler>,
		)>,
	) -> Self {
		let (entity, route_info, route_scene, handler) =
			query.get(*entity).expect("entity has no route info");

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
		let template_flags = template_flags.as_ref().map(|res| (*res).clone());
		let layers = parents
			.iter_ancestors_inclusive(entity)
			.filter_map(|e| layers.get(e).ok().cloned())
			.collect::<Vec<_>>();


		Self {
			workspace_config,
			html_constants,
			route_info: route_info.clone(),
			handler,
			route_scene,
			layers,
			template_flags,
		}
	}

	pub async fn call(self, request: Request) -> AppResult<Response> {
		let start_time = std::time::Instant::now();

		let mut world = {
			let mut app = App::new();
			app.add_plugins((AppRouterPlugin, TemplatePlugin))
				.insert_resource(self.workspace_config)
				.insert_resource(self.html_constants);
			if let Some(flags) = self.template_flags {
				app.insert_resource(flags);
			}

			// #[cfg(all(not(test), feature = "build"))]
			// app.add_plugins(beet_build::prelude::BuildPlugin::default());

			for layer in self.layers {
				layer.add_to_app(&mut app);
			}
			std::mem::take(app.world_mut())
		};

		world.insert_resource(request);

		if let Some(route_scene) = self.route_scene {
			world.load_scene(route_scene.ron).map_err(|err| {
				AppError::bad_request(format!("Failed to load scene: {err}"))
			})?;
		}

		world.try_run_schedule(BeforeRoute).ok();

		if let Some(handler) = self.handler {
			// if handler errors it is inserted into RouteHandlerOutput
			world = handler.run(world).await
		}

		world.try_run_schedule(AfterRoute).ok();
		if !world.contains_resource::<Response>() {
			world.try_run_schedule(CollectResponse).ok();
		}

		let response = world.remove_resource::<Response>().unwrap_or_default();

		debug!("Route handler completed in: {:.2?}", start_time.elapsed());

		Ok(response)
	}
}
