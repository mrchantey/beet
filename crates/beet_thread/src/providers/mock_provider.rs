//! Mock post streamer for testing.
//!
//! [`MockPostStreamer`] simulates an AI model for testing via the [`PostStreamer`] API:
//! - With tools, it calls the first tool with default argument values
//! - Without tools, it echoes the input prefixed with "you said:"
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

/// Counter for generating unique IDs.
static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_id(prefix: &str) -> String {
	format!("{}_{}", prefix, ID_COUNTER.fetch_add(1, Ordering::SeqCst))
}

/// A mock post streamer for testing tool-calling workflows.
///
/// ## Behavior
///
/// - **With tools**: Calls the first tool with default values generated from
///   the parameter schema (strings become "", integers become 0, etc.)
/// - **Without tools**: Returns the user's input prefixed with "you said:"
#[derive(Debug, Clone, Default, PartialEq, Eq, Component)]
#[component(on_add = on_add)]
pub struct MockPostStreamer {
	/// Optional custom response text, overrides default echo behavior.
	pub custom_response: Option<String>,
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).insert(
		Action::<(), Outcome>::new_async(
			post_streamer_action::<MockPostStreamer>,
		),
	);
}

impl MockPostStreamer {
	/// Creates a new mock streamer.
	pub fn new() -> Self { Self::default() }

	/// Creates a mock streamer that always returns the specified text.
	pub fn with_response(text: impl Into<String>) -> Self {
		Self {
			custom_response: Some(text.into()),
		}
	}

	/// Generates default arguments for a tool based on its parameter schema.
	fn generate_default_arguments(schema: &Schema) -> String {
		// Convert to serde_json::Value for traversal and serialization
		let json_schema = schema.clone().into_inner().into_json();
		let Some(properties) =
			json_schema.get("properties").and_then(|p| p.as_object())
		else {
			return "{}".to_string();
		};

		let mut args = serde_json::Map::new();
		for (name, field_schema) in properties {
			let default_value = Self::default_value_for_schema(field_schema);
			args.insert(name.clone(), default_value);
		}
		serde_json::to_string(&serde_json::Value::Object(args))
			.unwrap_or_else(|_| "{}".to_string())
	}

	/// Generates a default value based on JSON Schema type.
	fn default_value_for_schema(
		schema: &serde_json::Value,
	) -> serde_json::Value {
		match schema
			.get("type")
			.and_then(|t| t.as_str())
			.unwrap_or("string")
		{
			"string" => serde_json::Value::String(String::new()),
			"integer" => serde_json::Value::Number(0.into()),
			"number" => serde_json::json!(0.0),
			"boolean" => serde_json::Value::Bool(false),
			"array" => serde_json::Value::Array(vec![]),
			"object" => serde_json::Value::Object(serde_json::Map::new()),
			"null" => serde_json::Value::Null,
			_ => serde_json::Value::String(String::new()),
		}
	}
}

impl PostStreamer for MockPostStreamer {
	fn provider_slug(&self) -> &str { "mock" }
	fn model_slug(&self) -> &str { "mock-model" }

	fn stream_posts(
		&self,
		caller: AsyncEntity,
	) -> BoxedFuture<'_, Result<PostStream>> {
		let custom_response = self.custom_response.clone();

		Box::pin(async move {
			let (agent_id, thread_id, last_user_text, first_tool) = caller
				.with_state::<ThreadQuery, _>(
					|actor_entity,
					 query|
					 -> Result<(
						ActorId,
						ThreadId,
						String,
						Option<(String, Schema)>,
					)> {
						let (_, thread, window) =
							query.thread_and_window(actor_entity)?;
						let agent_id = query.actor_id(actor_entity)?;

						// Extract last user text
						let last_text = window
							.post_views()
							.filter(|view| view.actor.kind() == ActorKind::User)
							.last()
							.and_then(|view| view.post.body_str().ok())
							.unwrap_or_default()
							.to_string();

						// Get first tool if any
						let first_tool = query
							.tools(actor_entity)
							.into_iter()
							.find_map(|(_, tool_def)| match tool_def {
								ToolDefinition::Function(func) => Some((
									func.path().to_string(),
									func.params_schema().clone(),
								)),
								_ => None,
							});

						(agent_id, thread.id(), last_text, first_tool).xok()
					},
				)
				.await??;

			let response_id = next_id("mock-resp");

			// Build the response partial
			let posts = if let Some((name, params)) = first_tool {
				let arguments = Self::generate_default_arguments(&params);
				vec![PostPartial {
					key: PostPartialKey::Single {
						responses_id: next_id("fc"),
					},
					status: PostStatus::Completed,
					content: PartialContent::FunctionCall {
						name,
						call_id: next_id("call"),
						arguments,
					},
				}]
			} else {
				let text = custom_response
					.unwrap_or_else(|| format!("you said: {}", last_user_text));
				vec![PostPartial {
					key: PostPartialKey::Content {
						responses_id: response_id.clone(),
						content_index: 0,
					},
					status: PostStatus::Completed,
					content: PartialContent::TextDone {
						text,
						logprobs: Vec::new(),
					},
				}]
			};

			let partial = ResponsePartial {
				response_id: response_id.clone(),
				response_stored: false,
				status: ResponseStatus::Completed,
				token_usage: Some(TokenUsage {
					input_tokens: 10,
					output_tokens: 10,
					total_tokens: 20,
					cached_input_tokens: None,
					reasoning_tokens: None,
				}),
				posts,
			};

			let stream: ResPartialStream =
				Box::pin(futures::stream::once(async move { Ok(partial) }));

			PostStream::new("mock", "mock-model", agent_id, thread_id, stream)
				.xok()
		})
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	async fn echoes_input_without_tools() {
		run_oneshot(children![
			(Actor::user(), children![Post::spawn("Hello world!")]),
			(Actor::agent(), MockPostStreamer::default()),
		])
		.await
		.unwrap()
		.into_iter()
		.find(|post| post.intent().is_display())
		.unwrap()
		.to_string()
		.xpect_eq("you said: Hello world!");
	}

	#[beet_core::test]
	async fn calls_first_tool_when_present() {
		let tool: ToolDefinition = FunctionToolDefinition::new(
			"greet",
			"Greet someone",
			serde_json::json!({
				"type": "object",
				"properties": {
					"name": { "type": "string" },
					"age": { "type": "integer" }
				}
			}),
		)
		.into();

		let (name, args) = run_oneshot(children![
			(Actor::user(), children![Post::spawn("Greet someone")]),
			(Actor::agent(), MockPostStreamer::default(), children![tool]),
		])
		.await
		.unwrap()
		.into_iter()
		.filter_map(|post| match post.as_agent_post() {
			AgentPost::FunctionCall(fc) => {
				Some((fc.name().to_string(), fc.arguments().to_string()))
			}
			_ => None,
		})
		.next()
		.unwrap();

		name.xpect_eq("greet");
		let parsed: serde_json::Value = serde_json::from_str(&args).unwrap();
		parsed["name"].as_str().unwrap().xpect_eq("");
		parsed["age"].as_i64().unwrap().xpect_eq(0);
	}

	#[beet_core::test]
	async fn custom_response_overrides_echo() {
		run_oneshot(children![
			(Actor::user(), children![Post::spawn("Hello!")]),
			(
				Actor::agent(),
				MockPostStreamer::with_response("Custom answer")
			),
		])
		.await
		.unwrap()
		.into_iter()
		.find(|post| post.intent().is_display())
		.unwrap()
		.to_string()
		.xpect_eq("Custom answer");
	}
}
