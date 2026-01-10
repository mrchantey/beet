use crate::prelude::*;
use beet::prelude::*;

/// The plugin added to the router app
pub fn server_plugin(app: &mut App) {
	app.add_plugins((
		MinimalPlugins,
		RouterPlugin,
		AgentPlugin,
		// DebugFlowPlugin::default(),
	))
	.world_mut()
	.spawn(default_router(
		|| EndWith(Outcome::Pass),
		|| {
			(InfallibleSequence, children![
				pages_routes(),
				docs_routes(),
				blog_routes(),
				actions_routes(),
				beet_design::mockups::mockups_routes(),
				article_layout_middleware("docs"),
				article_layout_middleware("blog"),
				image_generator(),
			])
		},
		|| EndWith(Outcome::Pass),
	));
}

#[allow(unused)]
fn image_generator() -> impl Bundle {
	EndpointBuilder::default()
		.with_path("generate_image")
		.with_handler(async |request: Request, action: AsyncEntity| -> () {
			// let request = world.remove_resource::<Request>().unwrap();
			let content =
				Json::<ContentVec>::from_request(request).await.unwrap();
			let agent =
				GeminiAgent::from_env().with_model(GEMINI_2_5_FLASH_IMAGE);

			let message = session_ext::message(content.0);
			let session = session_ext::user_message_session(agent, message);
			// agents.entity(entity).spawn_child(session).await;
			// AsyncRunner::flush_async_tasks(&mut world).await;
			todo!("run session and await outcome");

			// let out = action
			// 	.run_system_cached(session_ext::collect_output)
			// 	.await
			// 	.unwrap();
			// Json(out)
		})
}
