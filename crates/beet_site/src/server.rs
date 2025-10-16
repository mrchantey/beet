use crate::prelude::*;
use beet::prelude::*;

/// The plugin added to the router app
pub fn server_plugin(app: &mut App) {
	app.add_plugins(AgentPlugin).world_mut().spawn((
		RouteServer,
		InfallibleSequence,
		children![
			pages_routes(),
			docs_routes(),
			blog_routes(),
			actions_routes(),
			beet_design::mockups::mockups_routes(),
			article_layout_middleware().with_path("docs"),
			article_layout_middleware().with_path("blog"),
			image_generator(),
			(Fallback, children![html_bundle_to_response()])
		],
	));
}

#[allow(unused)]
fn image_generator() -> impl Bundle {
	EndpointBuilder::default()
		.with_path("generate_image")
		.with_handler(async |request: Request, cx: EndpointContext| {
			// let request = world.remove_resource::<Request>().unwrap();
			let content =
				Json::<ContentVec>::from_request(request).await.unwrap();
			let agent =
				GeminiAgent::from_env().with_model(GEMINI_2_5_FLASH_IMAGE);

			let message = session_ext::message(content.0);
			let session = session_ext::user_message_session(agent, message);
			cx.exchange().spawn_child(session).await;
			// AsyncRunner::flush_async_tasks(&mut world).await;
			todo!("run session and await outcome");

			let out = cx
				.run_system_cached(session_ext::collect_output)
				.await
				.unwrap();
			Json(out)
		})
}
