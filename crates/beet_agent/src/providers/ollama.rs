use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;
use serde_json::json;

const DEFAULT_MODEL: &str = "functiongemma:270m-it-fp16";

#[derive(Component)]
#[require(AgentRole)]
#[component(on_add=on_add)]
pub struct OllamaAgent {
	base_url: String,
	/// Model used for chat completions
	completion_model: String,
	/// Reserved for future Ollama tools usage
	#[allow(unused)]
	tools: Vec<Value>,
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.insert(OnSpawn::observe(ollama_message_request));
}

impl OllamaAgent {
	/// Create a new Ollama client with default localhost URL
	pub fn new() -> Self {
		Self {
			base_url: "http://localhost:11434".to_string(),
			completion_model: DEFAULT_MODEL.into(),
			tools: Vec::new(),
		}
	}

	/// Create from environment variable OLLAMA_BASE_URL or use default
	pub fn from_env() -> Self {
		let base_url = env_ext::var("OLLAMA_BASE_URL")
			.unwrap_or_else(|_| "http://localhost:11434".to_string());
		Self {
			base_url,
			completion_model: DEFAULT_MODEL.into(),
			tools: Vec::new(),
		}
	}

	pub fn with_model(mut self, model: impl Into<String>) -> Self {
		self.completion_model = model.into();
		self
	}

	pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
		self.base_url = base_url.into();
		self
	}

	/// Currently a no-op; reserved to keep API parity with other providers
	pub fn with_tool(self, _tool: impl Into<CommonTool>) -> Self { self }

	fn chat_req(&self, messages: &Vec<Value>) -> Result<Request> {
		let url = format!("{}/api/chat", self.base_url);
		Request::post(url)
			.with_json_body(&json! {{
				"model": self.completion_model,
				"messages": messages,
				"stream": true,
			}})?
			.xok()
	}
}

// https://github.com/ollama/ollama/blob/main/docs/api.md#generate-a-chat-completion
fn ollama_message_request(
	ev: On<MessageRequest>,
	query: Query<&OllamaAgent>,
	mut commands: AsyncCommands,
	cx: SessionParams,
) -> Result {
	let actor = ev.event_target();
	let provider = query.get(actor)?;

	let messages = cx
		.collect_messages(actor)?
		.into_iter()
		.map(|item| {
			let is_self = item.actor.entity == actor;
			let role = if is_self { "assistant" } else { "user" };

			let content_parts = item
				.content
				.into_iter()
				.map(|part| match part {
					ContentView::Text(content) => content.0.clone(),
					ContentView::File(file) => match &file.data {
						FileData::Utf8(utf8) => format!(
							"<file src={}>{}</file>",
							file.filename.to_string_lossy(),
							utf8
						),
						FileData::Base64(_) | FileData::Uri(_) => {
							format!(
								"[File: {}]",
								file.filename.to_string_lossy()
							)
						}
					},
				})
				.collect::<Vec<_>>()
				.join("\n");

			json!({
				"role": role,
				"content": content_parts
			})
		})
		.collect::<Vec<_>>();

	if messages.is_empty() {
		bevybail!("cannot send request with no messages");
	}

	let req = provider.chat_req(&messages)?;

	commands.run_local(async move |queue| {
		let mut spawner = MessageSpawner::spawn(queue.clone(), actor).await?;

		let mut body_stream = req.send().await?.body;

		let mut dump = Vec::new();
		let text_content_id = 0;
		let mut buffer = String::new();

		loop {
			let chunk_opt = body_stream.next().await?;
			let chunk = match chunk_opt {
				Some(c) => c,
				None => break,
			};
			let chunk_str = String::from_utf8_lossy(&chunk);
			buffer.push_str(&chunk_str);

			// Process complete lines
			while let Some(newline_pos) = buffer.find('\n') {
				let line = buffer[..newline_pos].to_string();
				buffer = buffer[newline_pos + 1..].to_string();

				if line.trim().is_empty() {
					continue;
				}

				let Ok(body) = serde_json::from_str::<Value>(&line) else {
					eprintln!("failed to parse line as json: {}", line);
					continue;
				};

				dump.push(body.clone());

				// Handle errors
				if let Some(error) = body["error"].as_str() {
					bevybail!("Ollama API error: {}", error);
				}

				// Check if streaming is done
				let done = body["done"].as_bool().unwrap_or(false);

				// Process message content (both thinking and content fields)
				if let Some(message) = body["message"].as_object() {
					// Some models use "thinking" field for reasoning
					if let Some(thinking) = message["thinking"].as_str() {
						if !thinking.is_empty() {
							spawner
								.add_or_delta(text_content_id, thinking)
								.await?;
						}
					}
					// Regular content field
					if let Some(content) = message["content"].as_str() {
						if !content.is_empty() {
							spawner
								.add_or_delta(text_content_id, content)
								.await?;
						}
					}
				}

				// When done, extract token usage if available and exit both loops
				if done {
					if let (Some(prompt_tokens), Some(completion_tokens)) = (
						body["prompt_eval_count"].as_u64(),
						body["eval_count"].as_u64(),
					) {
						let prompt = prompt_tokens;
						let completion = completion_tokens;
						queue
							.entity(actor)
							.with_then(move |mut entity| {
								super::shared::update_token_usage(
									&mut entity,
									prompt,
									completion,
								);
							})
							.await;
					}
					// Break from both the line processing and chunk reading loops
					buffer.clear();
					break;
				}
			}
			// If buffer was cleared, we're done
			if buffer.is_empty() {
				break;
			}
		}

		super::shared::write_dump(&dump).await?;
		spawner.finish_message().await?;
		Ok(())
	});

	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[sweet::test]
	#[ignore = "sweet test timeout too short for ollama"]
	async fn text_to_text() {
		test_utils::text_to_text(OllamaAgent::from_env()).await;
	}

	#[sweet::test]
	#[ignore = "sweet test timeout too short for ollama"]
	async fn textfile_to_text() {
		test_utils::textfile_to_text(OllamaAgent::from_env()).await;
	}

	#[sweet::test]
	#[ignore = "requires specific model and longer timeout"]
	async fn text_to_text_qwen() {
		test_utils::text_to_text(
			OllamaAgent::from_env()
				.with_model("huihui_ai/qwen3-abliterated:14b"),
		)
		.await;
	}
}
