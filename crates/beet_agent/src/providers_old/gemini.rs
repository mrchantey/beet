use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;
use serde_json::json;

const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
pub const GEMINI_2_5_FLASH: &str = "gemini-2.5-flash";
pub const GEMINI_2_5_FLASH_IMAGE: &str = "gemini-2.5-flash-image-preview";

#[derive(Component)]
#[require(AgentRole)]
#[component(on_add=on_add)]
pub struct GeminiAgent {
	api_key: String,
	/// Model used
	completion_model: String,
	/// Reserved for future Gemini tools usage (eg. function declarations)
	#[allow(unused)]
	tools: Vec<Value>,
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.insert(OnSpawn::observe(gemini_message_request));
}

impl GeminiAgent {
	pub fn with_model(mut self, model: impl Into<String>) -> Self {
		self.completion_model = model.into();
		self
	}


	/// Create a new Gemini client from environment variables.
	/// ## Panics
	/// If the GEMINI_API_KEY environment variable is not set.
	pub fn from_env() -> Self {
		Self {
			api_key: env_ext::var("GEMINI_API_KEY").unwrap(),
			completion_model: GEMINI_2_5_FLASH.into(),
			tools: Vec::new(),
		}
	}

	/// Currently a no-op; reserved to keep API parity with OpenAIAgent.
	pub fn with_tool(self, _tool: impl Into<CommonTool>) -> Self { self }

	fn stream_req(&self, contents: &Vec<Value>) -> Result<Request> {
		let url = format!(
			"{BASE_URL}/models/{}:streamGenerateContent?alt=sse",
			self.completion_model
		);
		let body = json!({
			"contents": contents,
		});
		Request::post(url)
			.with_header("x-goog-api-key", &self.api_key)
			.with_json_body(&body)?
			.xok()
	}
}

// https://ai.google.dev/api/generate-content#method:-models.streamgeneratecontent
fn gemini_message_request(
	ev: On<MessageRequest>,
	query: Query<&GeminiAgent>,
	mut commands: AsyncCommands,
	cx: SessionParams,
) -> Result {
	let actor = ev.event_target();
	let provider = query.get(actor)?;

	let contents = cx
		.collect_messages(actor)?
		.into_iter()
		.map(|item| {
			let is_self = item.actor.entity == actor;
			// Gemini expects roles of "user" or "model"
			let role = if is_self { "model" } else { "user" };

			let parts = item
				.content
				.into_iter()
				.map(|part| match part {
					ContentView::Text(content) => json!({
						"text": content.0,
					}),
					ContentView::File(file) => {
						// Prefer inlineData if we have base64 or data: URI; otherwise fall back to fileData with a URL
						match &file.data {
							FileData::Base64(b64) => json!({
								"inlineData": {
									"mimeType": file.mime_type,
									"data": b64
								}
							}),
							FileData::Utf8(utf8) => {
								// Represent non-binary file as text content
								json!({
									"text": format!("<file src={}>{}</file>", file.filename.to_string_lossy(), utf8),
								})
							}
							FileData::Uri(uri) => {
								if uri.starts_with("data:") {
									let b64 = uri.splitn(2, ',').nth(1).unwrap_or_default();
									json!({
										"inlineData": {
											"mimeType": file.mime_type,
											"data": b64
										}
									})
								} else {
									json!({
										"fileData": {
											"mimeType": file.mime_type,
											"fileUri": uri
										}
									})
								}
							}
						}
					}
				})
				.collect::<Vec<_>>();

			json!({
				"role": role,
				"parts": parts
			})
		})
		.collect::<Vec<_>>();

	if contents.is_empty() {
		bevybail!("cannot send request with no contents");
	}

	let req = provider.stream_req(&contents)?;

	commands.run_local(async move |queue| {
		let mut spawner = MessageSpawner::spawn(queue.clone(), actor).await?;

		let mut stream = req.send().await?.event_source().await?;

		let mut input_tokens: u64 = 0;
		let mut output_tokens: u64 = 0;
		let mut dump = Vec::new();

		while let Some(ev) = stream.next().await {
			let ev = ev?;
			let Ok(body) = serde_json::from_str::<serde_json::Value>(&ev.data)
			else {
				eprintln!("failed to parse event data as json: {:?}", ev);
				continue;
			};
			// println!("Gemini event: {body:#?}");

			dump.push(body.clone());


			// Update token usage if present
			if let Some(usage) = body["usageMetadata"].as_object() {
				input_tokens = usage["promptTokenCount"].to_u64()?;
				output_tokens = usage["candidatesTokenCount"].to_u64()?;
			}

			// Streamed candidates with parts
			let candidates = body["candidates"].to_array()?;
			if candidates.is_empty() {
				bevybail!("Gemini Error: {}", body.to_string());
			}
			let cand = &candidates[0];
			let parts = cand["content"]["parts"].to_array()?;
			let text_content_id = 0;
			let image_content_id = 1;
			for part in parts.iter() {
				if let Some(text) = part["text"].as_str() {
					spawner.add_or_delta(text_content_id, text).await?;
				} else if let Some(inline) = part["inlineData"].as_object() {
					let mime = inline["mimeType"].to_str()?;
					let data = inline["data"].to_str()?;
					let ext = mime.split('/').nth(1).unwrap();
					let content = FileContent::new_b64(&"foobar", ext, data);
					spawner.insert(image_content_id, content).await?;
				} else {
					bevybail!("Unhandled Gemini part: {}", part.to_string());
				}
			}
		}

		// Persist usage and finalize the message
		queue
			.entity(actor)
			.with_then(move |mut entity| {
				super::shared::update_token_usage(
					&mut entity,
					input_tokens,
					output_tokens,
				);
			})
			.await;

		super::shared::write_dump(&dump).await?;
		spawner.finish_message().await?;
		Ok(())
	});

	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[beet_core::test]
	async fn text_to_text() {
		test_utils::text_to_text(GeminiAgent::from_env()).await;
	}

	#[beet_core::test]
	async fn textfile_to_text() {
		test_utils::textfile_to_text(GeminiAgent::from_env()).await;
	}

	#[beet_core::test]
	async fn image_to_text() {
		test_utils::image_to_text(GeminiAgent::from_env()).await;
	}

	#[beet_core::test]
	async fn text_to_image() {
		test_utils::text_to_image(
			GeminiAgent::from_env().with_model(GEMINI_2_5_FLASH_IMAGE),
		)
		.await;
	}
}
