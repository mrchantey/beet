use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde_json::json;

const OPENAI_API_BASE_URL: &str = "https://api.openai.com/v1";
const GPT_5_MINI: &str = "gpt-5-mini";

#[derive(Component)]
#[require(Agent)]
pub struct OpenAiProvider {
	api_key: String,
	/// Model used for chat completions, defaults to [`GPT_5_MINI`]
	completion_model: String,
	/// The id of the previous response
	prev_response_id: Option<String>,
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
		}
	}

	fn completions_req(
		&self,
		input: &Vec<serde_json::Value>,
	) -> Result<Request> {
		let url = format!("{OPENAI_API_BASE_URL}/responses");
		Request::post(url)
			.with_auth_bearer(&self.api_key)
			.with_json_body(&json! {{
				"model": self.completion_model,
				"stream": true,
				"input": input,
				"previous_response_id": self.prev_response_id
			}})?
			.xok()
	}
}

pub fn open_ai_provider() -> impl Bundle {
	(
		OpenAiProvider::from_env(),
		EntityObserver::new(handle_openai_request),
	)
}

fn handle_openai_request(
	trigger: Trigger<ContentBroadcast<ContentEnded>>,
	query: Query<&OpenAiProvider>,
	mut commands: Commands,
	cx: SessionContext,
) -> Result {
	let ContentBroadcast {
		session,
		owner: content_owner,
		..
	} = trigger.event().clone();

	let member_ent = trigger.target();
	if member_ent == content_owner {
		// println!("ignoring own content");
		return Ok(());
	}
	let input = cx
		.collect_content_relative(session, member_ent)?
		.into_iter()
		.map(|item| {
			let role = match item.role {
				Role::This => "assistant",
				Role::Developer => "developer",
				Role::Other => "user",
			};
			let content = match item.content {
				Content::Text(content) => &content.0,
			};
			json! {{
				"role": role,
				"content": content
			}}
		})
		.collect::<Vec<_>>();
	assert!(input.len() > 0, "cannot send request with no input");
	let provider = query.get(member_ent)?;
	let req = provider.completions_req(&input)?;

	commands.run_system_cached_with(
		AsyncTask::spawn_with_queue_unwrap,
		async move |queue| {
			let mut stream = req.send().await?.event_source().await?;

			// map of content_index to entity
			let mut content_map = HashMap::new();

			while let Some(ev) = stream.next().await {
				let ev = ev?;
				if let Ok(body) =
					serde_json::from_str::<serde_json::Value>(&ev.data)
				{
					// println!("event: {body:#?}");
					// https://platform.openai.com/docs/api-reference/responses_streaming/response
					match body.field_str("type")? {
						"response.created" => {
							// let id = body["response"]["id"].to_str()?;
						}
						"response.in_progress" => {}
						"response.output_item.added" => {}
						"response.content_part.added" => {
							match body["part"]["type"].to_str()? {
								"output_text" => {
									let index =
										body["content_index"].to_u64()?;
									if content_map.contains_key(&index) {
										bevybail!(
											"Duplicate output index: {index}"
										);
									} else {
										let entity = queue
											.spawn_then(text_content(
												session, member_ent, "",
											))
											.await;
										content_map.insert(index, entity);
									}
								}
								_ => {}
							}
						}
						"response.output_text.delta" => {
							let index = body["content_index"].to_u64()?;
							let entity =
								content_map.get(&index).ok_or_else(|| {
									bevyhow!(
										"Missing entity for index: {index}"
									)
								})?;
							let new_text = body["delta"].to_str()?.to_string();
							queue
								.entity(*entity)
								.trigger(ContentTextDelta::new(new_text));
						}
						"response.output_text.done" => {}
						"response.content_part.done" => {
							let index = body["content_index"].to_u64()?;
							let entity =
								content_map.get(&index).ok_or_else(|| {
									bevyhow!(
										"Missing entity for index: {index}"
									)
								})?;
							queue.entity(*entity).trigger(ContentEnded);
						}
						"response.output_item.done" => {}
						"response.completed" => {
							let input_tokens =
								body["response"]["usage"]["input_tokens"]
									.to_u64()?;
							let output_tokens =
								body["response"]["usage"]["output_tokens"]
									.to_u64()?;
							let id =
								body["response"]["id"].to_str()?.to_string();

							queue
								.entity(member_ent)
								.get_mut::<OpenAiProvider>(
									move |mut provider| {
										provider.prev_response_id = Some(id);
									},
								)
								.get_mut::<TokenUsage>(move |mut tokens| {
									tokens.input_tokens += input_tokens;
									tokens.output_tokens += output_tokens;
								})
								.trigger(ResponseComplete);
						}
						_ => {}
					}
					// return Ok(());
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
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		dotenv::dotenv().ok();

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin, AgentPlugin));

		let mut session = SessionBuilder::from_app(&mut app);
		let user = session.add_member(User);
		let _agent = session.add_member(open_ai_provider());
		session.add_content(user, "what is 2 + 4?");

		app.add_observer(
			|ev: Trigger<ResponseComplete>,
			 mut commands: Commands,
			 text: Query<&TextContent>,
			 query: Query<(&TokenUsage, &OwnedContent)>| {
				let (_tokens, content) = query.get(ev.target()).unwrap();
				text.get(content[0]).unwrap().0.xref().xpect_contains("6");
				commands.send_event(AppExit::Success);
			},
		);

		app.run_async(AsyncChannel::runner_async)
			.await
			.into_result()
			.unwrap();
	}
}
