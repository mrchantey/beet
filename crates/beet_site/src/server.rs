use crate::prelude::*;
use beet::prelude::*;

/// The plugin added to the router app
pub fn server_plugin(app: &mut App) {
	app.insert_resource(Router::new(server_routes_plugin));
}

/// The plugin added to individual handler apps, not the router app
pub fn server_routes_plugin(app: &mut App) {
	app.add_plugins(AgentPlugin)
		.world_mut()
		.spawn(routes_bundle());
}

pub fn routes_bundle() -> impl Bundle {
	(RouterRoot, children![
		pages_routes(),
		docs_routes(),
		blog_routes(),
		actions_routes(),
		beet_design::mockups::mockups_routes(),
		(PathFilter::new("docs"), article_layout_middleware()),
		(PathFilter::new("blog"), article_layout_middleware()),
		image_generator()
	])
}



fn image_generator() -> impl Bundle {
	(
		PathFilter::new("generate_image"),
		RouteHandler::layer_async(async |mut world, _entity| {
			let request = world.remove_resource::<Request>().unwrap();
			let content: Json<ContentVec> = request.try_into().unwrap();
			let agent =
				GeminiAgent::from_env().with_model(GEMINI_2_5_FLASH_IMAGE);

			let message = session_ext::message(content.0);
			let session = session_ext::user_message_session(agent, message);
			let session = world.spawn(session).id();
			AsyncRunner::flush_async_tasks(&mut world).await;

			let out = world
				.run_system_cached(session_ext::collect_output)
				.unwrap();
			let response: Response = Json(out).try_into().unwrap();
			world.insert_resource(response);
			world.despawn(session);
			println!("DONE!");

			world
		}),
	)
}
