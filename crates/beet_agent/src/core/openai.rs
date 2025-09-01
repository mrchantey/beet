use crate::prelude::*;
use beet_core::bevybail;
use beet_core::bevyhow;
use beet_net::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use serde_json::json;

const OPENAI_API_BASE_URL: &str = "https://api.openai.com/v1";
const GPT_5_MINI: &str = "gpt-5-mini";

pub struct OpenAiProvider {
	api_key: String,
	/// Model used for chat completions, defaults to [`GPT_5_MINI`]
	completion_model: String,
	/// Whether new requests should use the previous response id
	stateful: bool,
	prev_responses_id: Option<String>,
}

impl OpenAiProvider {
	/// Create a new OpenAI client from environment variables.
	/// ## Panics
	/// If the OPENAI_API_KEY environment variable is not set.
	pub fn from_env() -> Self {
		Self {
			api_key: std::env::var("OPENAI_API_KEY").unwrap(),
			completion_model: GPT_5_MINI.into(),
			stateful: false,
			prev_responses_id: None,
		}
	}

	fn completions_req(
		&self,
		req: CompletionsRequest,
		stream: bool,
	) -> Result<Request> {
		let url = format!("{OPENAI_API_BASE_URL}/responses");

		let messages = req
			.content
			.into_iter()
			.map(|content| match content {
				Content::System(InputContent::Text(TextContent(content))) => {
					json! {{
						"role":"system",
						"content": content
					}}
				}
				Content::User(InputContent::Text(TextContent(content))) => {
					json! {{
						"role":"user",
						"content": content
					}}
				}
				Content::Agent(OutputContent::Text(TextContent(content))) => {
					json! {{
						"role":"agent",
						"content": content
					}}
				}
			})
			.collect::<Vec<_>>();


		Request::post(url)
			.with_auth_bearer(&self.api_key)
			.with_json_body(&json! {{
				"model": self.completion_model,
				"stream": stream,
				"input": messages,
			}})?
			.xok()
	}
}


impl AgentProvider for OpenAiProvider {
	fn stream_completion(
		&self,
		request: CompletionsRequest,
	) -> SendBoxedFuture<Result<CompletionsStream>> {
		let req = self.completions_req(request, true);
		Box::pin(async move {
			let stream = req?.send().await?.event_source().await?;

			Ok(CompletionsStream {
				stream,
				map_data: Box::new(|ev| {
					use beet_utils::prelude::*;
					use serde_json::Value;

					// let data = ev.data;
					println!("event!: {}", ev.event);

					let Ok(json) = serde_json::from_str::<Value>(&ev.data)
					else {
						return Ok(CompletionsEvent::NoJson(ev));
					};

					match json.field_str("type")?.as_str() {
						"response.created" => {
							let session_id = json
								.field("response")?
								.field("id")?
								.to_string();
							Ok(CompletionsEvent::Created { session_id })
						}
						"response.in_progress" => {
							Ok(CompletionsEvent::Other { value: json })
						}
						_ => {
							Ok(CompletionsEvent::Other { value: json })
							// if let Some(data) =
							// 	json["choices"][0]["delta"]["content"].as_str()
							// {
							// 	Ok(CompletionsEvent::TextDelta(data.to_string()))
							// } else {
							// 	Ok(CompletionsEvent::Other(json))
							// }

							// bevybail!("unhandled type: {}", other)
						}
					}
				}),
			})
		})
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_net::prelude::*;
	use bevy::prelude::*;

	#[sweet::test]
	async fn works() {
		dotenv::dotenv().ok();

		let agent = Agent::new(OpenAiProvider::from_env());
		let mut completions = agent.stream_completion("foobar").await.unwrap();
		while let Some(event) = completions.next().await {
			match event.unwrap() {
				CompletionsEvent::Created { session_id } => {
					println!("Received created event: {}", session_id)
				}
				CompletionsEvent::TextDelta(text) => {
					println!("Received text: {}", text)
				}
				CompletionsEvent::Other { value } => {
					println!("Received other data: {:#?}", value)
				}
				CompletionsEvent::NoJson(ev) => {
					println!("NoJson with data: {:?}", ev)
				}
			}
		}
		// expect(true).to_be_false();
	}
}
