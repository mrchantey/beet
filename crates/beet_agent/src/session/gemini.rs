// use crate::prelude::*;
// use beet_core::prelude::*;
// use beet_net::prelude::*;
// use bevy::ecs::component::HookContext;
// use bevy::ecs::world::DeferredWorld;
// use bevy::prelude::*;
// use serde_json::Value;
// use serde_json::json;




// const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
// const GEMINI_2_5_FLASH_IMAGE: &str = "gemini-2.5-flash-image-preview";


// #[derive(Component)]
// #[require(Agent)]
// #[component(on_add=on_add)]
// pub struct GeminiAgent {
// 	api_key: String,
// 	/// Model used
// 	completion_model: String,
// 	/// The id of the previous response
// 	prev_response_id: Option<String>,
// 	tools: Vec<Value>,
// }

// fn on_add(mut world: DeferredWorld, cx: HookContext) {
// 	world
// 		.commands()
// 		.entity(cx.entity)
// 		.insert(EntityObserver::new(gemini_message_request));
// }

// // https://ai.google.dev/api/generate-content#method:-models.streamgeneratecontent
// fn gemini_message_request(
// 	trigger: Trigger<MessageRequest>,
// 	query: Query<&GeminiAgent>,
// 	mut commands: Commands,
// 	cx: SessionParams,
// ) -> Result {
// 	Ok(())
// }
