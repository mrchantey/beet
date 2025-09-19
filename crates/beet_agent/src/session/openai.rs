use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::ecs::component::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use serde_json::json;

const OPENAI_API_BASE_URL: &str = "https://api.openai.com/v1";
const GPT_5_MINI: &str = "gpt-5-mini";

#[derive(Component)]
#[require(Agent)]
#[component(on_add=on_add)]
pub struct OpenAiProvider {
	api_key: String,
	/// Model used for chat completions, defaults to [`GPT_5_MINI`]
	completion_model: String,
	/// The id of the previous response
	prev_response_id: Option<String>,
	tools: serde_json::Value,
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.insert(EntityObserver::new(start_openai_response));
}

impl OpenAiProvider {
	/// Create a new OpenAI client from environment variables.
	/// ## Panics
	/// If the OPENAI_API_KEY environment variable is not set.
	pub fn from_env() -> Self {
		Self {
			api_key: std::env::var("OPENAI_API_KEY").unwrap(),
			completion_model: GPT_5_MINI.into(),
			prev_response_id: None,
			tools: json!([]),
		}
	}
	pub fn with_image_gen(mut self) -> Self {
		self.tools = json!([{
			"type": "image_generation",
		}]);
		self
	}

	fn responses_req(&self, input: &Vec<serde_json::Value>) -> Result<Request> {
		// input.xprint_debug_formatted("content");
		let url = format!("{OPENAI_API_BASE_URL}/responses");
		Request::post(url)
			.with_auth_bearer(&self.api_key)
			.with_json_body(&json! {{
				"model": self.completion_model,
				"stream": true,
				"input": input,
				"tools": self.tools,
				"previous_response_id": self.prev_response_id
			}})?
			.xok()
	}
}

fn start_openai_response(
	trigger: Trigger<StartResponse>,
	query: Query<&OpenAiProvider>,
	mut commands: Commands,
	cx: SessionQuery,
) -> Result {
	let actor = trigger.target();
	let provider = query.get(actor)?;
	let input = cx
		.collect_messages(actor)?
		.into_iter()
		.map(|item| {
			let role = match item.role {
				RelativeRole::This => "assistant",
				RelativeRole::Developer => "developer",
				RelativeRole::Other => "user",
			};

			let content = item
				.content
				.into_iter()
				.map(|part| match part {
					ContentView::Text(content) => json!({
							"type":"input_text",
							"text": content.0,
					}),
					ContentView::File(file) if file.is_image() => json!({
						"type":"input_image",
						"image_url": file.into_url(),
					}),
					ContentView::File(file) => match &file.data {
						// only pdf file type supported
						FileData::Utf8(utf8) => json!({
							"type":"input_text",
							"text": format!("<file src={}>{}</file>", file.filename.to_string_lossy(), utf8),
						}),
						FileData::Uri(uri) => json!({
							"type":"input_file",
							"filename": file.filename.to_string_lossy(),
							"file_url": uri,
						}),
						FileData::Base64(_) => json!({
							"type":"input_file",
							"filename": file.filename.to_string_lossy(),
							"file_data": file.into_url(),
						}),
					},
				})
				.collect::<Vec<_>>();
			json! {{
				"role": role,
				"content": content
			}}
		})
		.collect::<Vec<_>>();
	assert!(input.len() > 0, "cannot send request with no input");
	let req = provider.responses_req(&input)?;

	commands.run_system_cached_with(
		AsyncTask::spawn_with_queue_unwrap,
		async move |queue| {
			let mut spawner =
				MessageSpawner::spawn(queue.clone(), actor).await?;

			let mut stream = req.send().await?.event_source().await?;

			let mut dump = Vec::new();

			while let Some(ev) = stream.next().await {
				let ev = ev?;
				if let Ok(body) =
					serde_json::from_str::<serde_json::Value>(&ev.data)
				{
					dump.push(body.clone());
					FsExt::write_async(
						AbsPathBuf::new_workspace_rel("dump.json").unwrap(),
						serde_json::to_string_pretty(&dump).unwrap(),
					)
					.await
					.unwrap();

					// body.xref().xprint_debug_formatted("response");
					// https://platform.openai.com/docs/api-reference/responses_streaming/response
					match body.field_str("type")? {
						"response.created" => {
							// let id = body["response"]["id"].to_str()?;
						}
						"response.in_progress" => {}
						"response.output_item.added" => {
							let id = body["item"]["id"].to_str()?.to_string();
							match body["item"]["type"].to_str()? {
								"reasoning" => {
									spawner
										.add(id, ReasoningContent::default())
										.await?;
								}
								"message" => {
									spawner
										.add(id, TextContent::default())
										.await?;
								}
								"image_generation_call" => {
									// we dont actually insert the file yet
									spawner.add(id, Content::default()).await?;
								}
								_ => {
									eprintln!(
										"unhandled item type: {}",
										body["item"]["type"].to_str()?
									);
								}
							}
						}
						"response.content_part.added" => {
							// see output_item.added
						}
						"response.output_text.delta" => {
							let id = body["item_id"].to_str()?.to_string();

							let new_text = body["delta"].to_str()?.to_string();
							spawner.text_delta(id, new_text).await?;
						}
						"response.image_generation_call.in_progress"
						| "response.image_generation_call.generating"
						| "response.image_generation_call.completed" => {}
						"response.image_generation_call.partial_image" => {
							let id = body["item_id"].to_str()?;
							let ext = body["output_format"].to_str()?;
							let b64 = body["partial_image_b64"].to_str()?;
							let content = FileContent::new_b64(id, ext, b64);
							spawner.insert(id.to_string(), content).await?;
						}
						"response.output_text.done" => {
							// see output_item.done
						}
						"response.content_part.done" => {
							// see output_item.done
						}
						"response.output_item.done" => {
							let id = body["item"]["id"].to_str()?.to_string();
							match body["item"]["type"].to_str()? {
								"image_generation_call" => {
									let ext = body["item"]["output_format"]
										.to_str()?;
									let b64 =
										body["item"]["result"].to_str()?;
									let content =
										FileContent::new_b64(&id, ext, b64);
									spawner
										.insert(id.to_string(), content)
										.await?;
								}
								_ => {}
							}
							spawner.finish_content(id).await?;
						}
						"response.completed" => {
							let input_tokens =
								body["response"]["usage"]["input_tokens"]
									.to_u64()?;
							let output_tokens =
								body["response"]["usage"]["output_tokens"]
									.to_u64()?;
							let id =
								body["response"]["id"].to_str()?.to_string();
							spawner.finish_message().await?;
							queue
								.entity(actor)
								.with(move |mut entity| {
									entity
										.get_mut::<OpenAiProvider>()
										.unwrap()
										.prev_response_id = Some(id);
									let mut tokens =
										entity.get_mut::<TokenUsage>().unwrap();
									tokens.input_tokens += input_tokens;
									tokens.output_tokens += output_tokens;
									entity.trigger(ResponseComplete);
								})
								.await;
						}
						"error" => {
							let message = body["error"]["message"].to_str()?;
							bevybail!("OpenAI API error: {message}");
						}
						other => {
							eprintln!(
								"unhandled event type: {other}\n{body:#?}"
							);
						}
					}
				};
			}
			Ok(())
		},
	);
	Ok(())
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	#[sweet::test]
	async fn text_to_text() {
		super::super::test::text_to_text(OpenAiProvider::from_env()).await;
	}

	#[sweet::test]
	async fn textfile_to_text() {
		super::super::test::textfile_to_text(OpenAiProvider::from_env()).await;
	}
	#[sweet::test]
	async fn image_to_text() {
		super::super::test::image_to_text(OpenAiProvider::from_env()).await;
	}
	#[sweet::test]
	async fn text_to_image() {
		super::super::test::text_to_image(
			OpenAiProvider::from_env().with_image_gen(),
		)
		.await;
	}
}
