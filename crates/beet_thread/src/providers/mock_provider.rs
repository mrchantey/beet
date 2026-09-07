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
/// - **With tools**: Calls the first tool. Its arguments are the next entry of
///   [`tool_arguments`](Self::tool_arguments) (cycled, looping) when that is set,
///   else the zero of its parameter schema ([`default_arguments`](Self::default_arguments)):
///   a required string becomes `""`, a required integer `0`, an optional field
///   is absent
/// - **Without tools**: Returns the user's input prefixed with "you said:"
#[derive(Debug, Clone, Default, PartialEq, Eq, Component)]
#[component(on_add = hook_ext::entity_hook(|entity| {
	entity.insert(Action::<(), Outcome>::new_async(
		post_streamer_action::<MockPostStreamer>,
	));
}))]
pub struct MockPostStreamer {
	/// Optional custom response text, overrides default echo behavior.
	pub custom_response: Option<String>,
	/// Canned function-call argument payloads (JSON), cycled one per call and
	/// looping, used instead of schema defaults when the model has a tool. Empty
	/// falls back to schema defaults. Lets a mock drive a real tool loop (eg the
	/// perceive-act `respond-multi-modal` fan-out) with lively values and no
	/// network call.
	pub tool_arguments: Vec<String>,
}

/// Cursor cycling [`MockPostStreamer::tool_arguments`] across calls.
static TOOL_ARG_CURSOR: AtomicU64 = AtomicU64::new(0);

impl MockPostStreamer {
	/// Creates a new mock streamer.
	pub fn new() -> Self { Self::default() }

	/// Creates a mock streamer that always returns the specified text.
	pub fn with_response(text: impl Into<String>) -> Self {
		Self {
			custom_response: Some(text.into()),
			..Default::default()
		}
	}

	/// Creates a mock streamer that cycles the given function-call argument
	/// payloads (JSON), one per call and looping, instead of schema defaults.
	pub fn with_tool_arguments(
		arguments: impl IntoIterator<Item = String>,
	) -> Self {
		Self {
			tool_arguments: arguments.into_iter().collect(),
			..Default::default()
		}
	}

	/// The default arguments for a tool call: the zero of its parameter schema,
	/// as a JSON object.
	///
	/// Reads the exported JSON Schema back into a [`ValueSchema`] and takes its
	/// [`default_value_in`](ValueSchema::default_value_in), so the mock answers
	/// whatever the schema layer already defines a zero to be: a field's own
	/// [`OnMissing::Default`], a declared numeric floor, an enum's first variant
	/// carrying its payload's zero.
	///
	/// The schema's `$defs` seed a [`SchemaRegistry`] first, so a nested composite
	/// property (which the exporter hoists into `$defs` and references with a
	/// `$ref`) resolves to that composite's real default rather than a wildcard.
	///
	/// # Errors
	///
	/// Returns an error if the tool's schema is not a readable JSON Schema.
	fn default_arguments(schema: &JsonSchema) -> Result<String> {
		let json = schema.clone().into_inner().into_json();
		let mut registry = SchemaRegistry::new();
		if let Some(defs) = json.get("$defs").and_then(|defs| defs.as_object())
		{
			for (name, def) in defs {
				registry
					.insert(name.as_str(), ValueSchema::from_json_value(def)?);
			}
		}
		ValueSchema::from_json_value(&json)?
			.default_value_in(SchemaResolver::default().with_schemas(&registry))
			.into_json()
			.xmap(|json| serde_json::to_string(&json))?
			.xok()
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
		let tool_arguments = self.tool_arguments.clone();

		Box::pin(async move {
			let (agent_id, thread_id, last_user_text, first_tool) = caller
				.with_state::<ThreadQuery, _>(
					|actor_entity,
					 query|
					 -> Result<(
						ActorId,
						ThreadId,
						String,
						Option<(String, JsonSchema)>,
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
				// cycle a canned payload if given, else schema defaults.
				let arguments = if tool_arguments.is_empty() {
					Self::default_arguments(&params)?
				} else {
					let index = TOOL_ARG_CURSOR.fetch_add(1, Ordering::SeqCst)
						as usize % tool_arguments.len();
					tool_arguments[index].clone()
				};
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
		Post::run_oneshot(children![
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
				},
				"required": ["name", "age"]
			}),
		)
		.into();

		let (name, args) = Post::run_oneshot(children![
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

	/// `with_tool_arguments` supplies the tool call's arguments verbatim instead of
	/// schema defaults, so a mock can drive a real tool loop with lively values.
	#[beet_core::test]
	async fn tool_arguments_override_schema_defaults() {
		let tool: ToolDefinition = FunctionToolDefinition::new(
			"greet",
			"Greet someone",
			serde_json::json!({
				"type": "object",
				"properties": { "name": { "type": "string" } }
			}),
		)
		.into();

		let (name, args) = Post::run_oneshot(children![
			(Actor::user(), children![Post::spawn("go")]),
			(
				Actor::agent(),
				MockPostStreamer::with_tool_arguments([
					r#"{"name":"Ada"}"#.to_string()
				]),
				children![tool]
			),
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
		// the canned payload, not the schema default (which would be an empty name).
		let parsed: serde_json::Value = serde_json::from_str(&args).unwrap();
		parsed["name"].as_str().unwrap().xpect_eq("Ada");
	}

	#[beet_core::test]
	async fn custom_response_overrides_echo() {
		Post::run_oneshot(children![
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

	/// The exporter hoists a named composite into `$defs` and references it with
	/// a `$ref`, so a nested struct property only has a real default if the
	/// `$defs` are resolved: unresolved it is a wildcard, ie null.
	#[beet_core::test]
	async fn nested_composite_default_resolves_through_defs() {
		let tool: ToolDefinition = FunctionToolDefinition::new(
			"place",
			"Place a marker",
			serde_json::json!({
				"type": "object",
				"properties": {
					"label": { "type": "string" },
					"at": { "$ref": "#/$defs/Point" }
				},
				"required": ["label", "at"],
				"$defs": {
					"Point": {
						"type": "object",
						"properties": {
							"x": { "type": "integer" },
							"y": { "type": "integer" }
						},
						"required": ["x", "y"]
					}
				}
			}),
		)
		.into();

		let args = Post::run_oneshot(children![
			(Actor::user(), children![Post::spawn("place one")]),
			(Actor::agent(), MockPostStreamer::default(), children![tool]),
		])
		.await
		.unwrap()
		.into_iter()
		.find_map(|post| match post.as_agent_post() {
			AgentPost::FunctionCall(fc) => Some(fc.arguments().to_string()),
			_ => None,
		})
		.unwrap();

		let parsed: serde_json::Value = serde_json::from_str(&args).unwrap();
		parsed["label"].as_str().unwrap().xpect_eq("");
		parsed["at"]["x"].as_i64().unwrap().xpect_eq(0);
		parsed["at"]["y"].as_i64().unwrap().xpect_eq(0);
	}
}
