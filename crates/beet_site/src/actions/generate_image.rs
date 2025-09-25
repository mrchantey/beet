use beet::prelude::*;

pub async fn post(
	In(content): In<ContentVec>,
	world: &mut World,
	entity: Entity,
) -> Result<FileContent, String> {
	unimplemented!()
	// // 1. construct the session with an image-capable model
	// let agent = GeminiAgent::from_env().with_model(GEMINI_2_5_FLASH_IMAGE);

	// let message = session_ext::message(content);
	// let session = session_ext::user_message_session(agent, message);

	// // 3. run the session and collect file outputs
	// let outputs = session_ext::run_and_collect_file(session).await;

	// // 4. find the first file content and return it
	// let output_file = outputs.into_iter().find_map(|content| match content {
	// 	ContentEnum::File(file) => Some(file),
	// 	_ => None,
	// });

	// output_file.ok_or_else(|| "No file content found".to_string())
}
