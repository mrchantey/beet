use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::ecs::component::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use serde_json::Value;
use serde_json::json;

const GPT_5_MINI: &str = "gpt-5-mini";

#[derive(Component)]
#[require(Agent)]
#[component(on_add=on_add)]
pub struct OpenAiAgent {
	api_key: String,
	base_url: String,
	/// Model used for chat completions, defaults to [`GPT_5_MINI`]
	completion_model: String,
	/// The id of the previous response
	prev_response_id: Option<String>,
	tools: Vec<Value>,
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.insert(EntityObserver::new(openai_message_request));
}

impl OpenAiAgent {
	/// Create a new OpenAI client from environment variables.
	/// ## Panics
	/// If the OPENAI_API_KEY environment variable is not set.
	pub fn from_env() -> Self {
		Self {
			api_key: std::env::var("OPENAI_API_KEY").unwrap(),
			completion_model: GPT_5_MINI.into(),
			base_url: "https://api.openai.com/v1".to_string(),
			prev_response_id: None,
			tools: Vec::new(),
		}
	}
	pub fn with_tool(mut self, tool: impl Into<CommonTool>) -> Self {
		let tool = tool.into();
		let json = match tool {
			CommonTool::GenerateImage(GenerateImage {
				background,
				quality,
				size,
				partial_images,
			}) => {
				json!({
					"type": "image_generation",
					"size": size
						.map(|size| size.to_string())
						.unwrap_or_else(|| "auto".to_string()),
					"background": background.to_string().to_lowercase(),
					"quality": quality.to_string().to_lowercase(),
					"partial_images": partial_images,
				})
			}
		};
		self.tools.push(json);
		self
	}

	fn responses_req(&self, input: &Vec<serde_json::Value>) -> Result<Request> {
		// input.xprint_debug_formatted("content");
		let url = format!("{}/responses", self.base_url);
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

fn openai_message_request(
	trigger: Trigger<MessageRequest>,
	query: Query<&OpenAiAgent>,
	mut commands: Commands,
	cx: SessionParams,
) -> Result {
	let actor = trigger.target();
	let provider = query.get(actor)?;
	let input = cx
		.collect_messages(actor)?
		.into_iter()
		.map(|item| {
			let is_self = item.actor.entity == actor;
			let role = if is_self {
				"assistant"
			} else {
				"user"
			};

			let content_type_prefix = if is_self {
				"output"}else{
				"input"
			};

			let content = item
				.content
				.into_iter()
				.map(|part| match part {
					ContentView::Text(content) => json!({
						"type": format!("{content_type_prefix}_text"),
							"text": content.0,
					}),
					ContentView::File(file) if file.is_image() => json!({
						"type":format!("{content_type_prefix}_image"),
						"image_url": file.into_url(),
					}),
					ContentView::File(file) => match &file.data {
						// only pdf file type supported
						FileData::Utf8(utf8) => json!({
							"type":format!("{content_type_prefix}_text"),
							"text": format!("<file src={}>{}</file>", file.filename.to_string_lossy(), utf8),
						}),
						FileData::Uri(uri) => json!({
							"type":format!("{content_type_prefix}_file"),
							"filename": file.filename.to_string_lossy(),
							"file_url": uri,
						}),
						FileData::Base64(_) => json!({
							"type":format!("{content_type_prefix}_file"),
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
				let Ok(body) = serde_json::from_str::<Value>(&ev.data) else {
					eprintln!(
						"failed to parse event data as json: {}",
						ev.data
					);
					continue;
				};
				dump.push(body.clone());
				fs_ext::write_async(
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
								spawner.add(id, TextContent::default()).await?;
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
								let ext =
									body["item"]["output_format"].to_str()?;
								let b64 = body["item"]["result"].to_str()?;
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
						let id = body["response"]["id"].to_str()?.to_string();
						queue
							.entity(actor)
							.with(move |mut entity| {
								entity
									.get_mut::<OpenAiAgent>()
									.unwrap()
									.prev_response_id = Some(id);
								let mut tokens =
									entity.get_mut::<TokenUsage>().unwrap();
								tokens.input_tokens += input_tokens;
								tokens.output_tokens += output_tokens;
							})
							.await;
						spawner.finish_message().await?;
					}
					"error" => {
						let message = body["error"]["message"].to_str()?;
						bevybail!("OpenAI API error: {message}");
					}
					other => {
						eprintln!("unhandled event type: {other}\n{body:#?}");
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
	use bevy::prelude::*;

	#[sweet::test]
	async fn text_to_text() {
		super::super::test::text_to_text(OpenAiAgent::from_env()).await;
	}

	#[sweet::test]
	async fn textfile_to_text() {
		super::super::test::textfile_to_text(OpenAiAgent::from_env()).await;
	}
	#[sweet::test]
	async fn image_to_text() {
		super::super::test::image_to_text(OpenAiAgent::from_env()).await;
	}
	#[sweet::test]
	async fn text_to_image() {
		super::super::test::text_to_image(OpenAiAgent::from_env().with_tool(
			GenerateImage {
				quality: ImageQuality::Low,
				..default()
			},
		))
		.await;
	}
}
