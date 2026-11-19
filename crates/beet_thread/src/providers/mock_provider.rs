//! Mock post streamer for testing.
//!
//! [`MockPostStreamer`] simulates an AI model for testing via the [`PostStreamer`] API:
//! - With a [`script`](MockPostStreamer::script), it plays one [`MockTurn`] per call
//! - Without one, it calls its first tool with default argument values, or with
//!   no tools echoes the input prefixed with "you said:"
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

/// A mock post streamer for testing tool-calling workflows, authorable in
/// markup so a scene runs its agents with no network:
///
/// ```bsx
/// <CreateActor name="Reader" kind="Agent" {MockPostStreamer{script:[
/// 	Call{tool:"read-blob", arguments:{path:"notes.md"}},
/// 	Text("done"),
/// ]}}>
/// 	<Router><StoreToolset read_only=true/></Router>
/// </CreateActor>
/// ```
///
/// ## Behavior
///
/// - **With a script**: plays its turns in order, one per call: a
///   [`MockTurn::Call`] calls the tool at its route path, a [`MockTurn::Text`]
///   answers.
///   Once the script is spent it answers with text, so a
///   [`RepeatWhileFunctionCallOutput`] loop ends, unless it
///   [`cycles`](Self::cycle), playing it again from the start
/// - **Without a script, with tools**: calls the first tool with the zero of
///   its parameter schema ([`default_arguments`](Self::default_arguments)): a
///   required string becomes `""`, a required integer `0`, an optional field
///   is absent
/// - **Without tools**: returns the user's input prefixed with "you said:"
///
/// Text answers are [`custom_response`](Self::custom_response) when set.
#[derive(Debug, Clone, Default, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = hook_ext::entity_hook(|entity| {
	entity.insert(Action::<(), Outcome>::new_async(
		post_streamer_action::<MockPostStreamer>,
	));
}))]
pub struct MockPostStreamer {
	/// Optional custom response text, overrides default echo behavior.
	pub custom_response: Option<String>,
	/// The turns played one per call, in order.
	pub script: Vec<MockTurn>,
	/// Play the script again from the start once it is spent, rather than
	/// answering with text.
	pub cycle: bool,
	/// The next turn of the script, advanced on every call.
	#[reflect(ignore)]
	cursor: usize,
}

/// One response of a scripted [`MockPostStreamer`].
#[derive(Debug, Clone, PartialEq, Reflect)]
pub enum MockTurn {
	/// Calls the agent's tool `tool` with `arguments` as its JSON body.
	Call {
		/// The tool's route path in the agent's url space, ie `read-blob`.
		tool: String,
		/// The call's arguments, a JSON object.
		arguments: Value,
	},
	/// Answers with this text.
	Text(String),
}

impl MockTurn {
	/// A call of `tool` with arguments serializing `arguments`.
	pub fn call(
		tool: impl Into<String>,
		arguments: impl Serialize,
	) -> Result<Self> {
		Self::Call {
			tool: tool.into(),
			arguments: Value::from_serde(arguments)?,
		}
		.xok()
	}
}

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

	/// Creates a mock streamer playing `script`, one turn per call.
	pub fn with_script(script: impl IntoIterator<Item = MockTurn>) -> Self {
		Self {
			script: script.into_iter().collect(),
			..Default::default()
		}
	}

	/// Creates a mock streamer playing `script` in a loop, never answering
	/// with text, ie a perceive-act agent cycling canned calls.
	pub fn cycling(script: impl IntoIterator<Item = MockTurn>) -> Self {
		Self {
			cycle: true,
			..Self::with_script(script)
		}
	}

	/// The scripted turn for this call, advancing the cursor on `caller`:
	/// `None` with no script, or once it is spent and does not cycle.
	async fn next_turn(caller: &AsyncEntity) -> Result<Option<MockTurn>> {
		caller
			.get_mut(|mut mock: Mut<MockPostStreamer>| {
				let len = mock.script.len();
				let index = match (len, mock.cycle) {
					(0, _) => return None,
					(len, true) => mock.cursor % len,
					(len, false) if mock.cursor >= len => return None,
					_ => mock.cursor,
				};
				mock.cursor += 1;
				mock.script[index].clone().xsome()
			})
			.await
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
		let scripted = !self.script.is_empty();

		Box::pin(async move {
			let turn = Self::next_turn(&caller).await?;
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
							.model_posts(thread, None)
							.into_iter()
							.filter(|view| view.actor.kind() == ActorKind::User)
							.last()
							.and_then(|view| view.post.body_str().ok())
							.unwrap_or_default()
							.to_string();

						// Get first tool if any
						let first_tool = query
							.tools(actor_entity)?
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

			// what this call answers: the script's turn, else with no script the
			// first tool's default call, else text
			// a call names its tool's route path, which the mock encodes as a
			// provider would
			let call = match turn {
				Some(MockTurn::Call { tool, arguments }) => {
					Some((tool, serde_json::to_string(&arguments)?))
				}
				Some(MockTurn::Text(text)) => {
					return Self::stream(
						agent_id,
						thread_id,
						response_id,
						text,
					);
				}
				None if scripted => None,
				None => first_tool
					.map(|(path, params)| -> Result<(String, String)> {
						(path, Self::default_arguments(&params)?).xok()
					})
					.transpose()?,
			};
			let posts = match call {
				Some((path, arguments)) => vec![PostPartial {
					key: PostPartialKey::Single {
						responses_id: next_id("fc"),
					},
					status: PostStatus::Completed,
					content: PartialContent::FunctionCall {
						name: FunctionToolDefinition::encode_name(&path),
						call_id: next_id("call"),
						arguments,
					},
				}],
				None => {
					let text = custom_response.unwrap_or_else(|| {
						format!("you said: {}", last_user_text)
					});
					return Self::stream(
						agent_id,
						thread_id,
						response_id,
						text,
					);
				}
			};
			Self::stream_partials(agent_id, thread_id, response_id, posts)
		})
	}
}

impl MockPostStreamer {
	/// A stream of one completed text answer.
	fn stream(
		agent_id: ActorId,
		thread_id: ThreadId,
		response_id: String,
		text: String,
	) -> Result<PostStream> {
		Self::stream_partials(agent_id, thread_id, response_id.clone(), vec![
			PostPartial {
				key: PostPartialKey::Content {
					responses_id: response_id,
					content_index: 0,
				},
				status: PostStatus::Completed,
				content: PartialContent::TextDone {
					text,
					logprobs: Vec::new(),
				},
			},
		])
	}

	/// A stream of one completed response carrying `posts`.
	fn stream_partials(
		agent_id: ActorId,
		thread_id: ThreadId,
		response_id: String,
		posts: Vec<PostPartial>,
	) -> Result<PostStream> {
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

		PostStream::new("mock", "mock-model", agent_id, thread_id, stream).xok()
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use beet_router::prelude::Router;

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
		.unwrap()
		.into();

		let (name, args) = Post::run_oneshot(children![
			(Actor::user(), children![Post::spawn("Greet someone")]),
			(Actor::agent(), MockPostStreamer::default(), children![(
				Router,
				children![tool]
			)]),
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

	/// A script plays its turns in order, calling tools by path with its own
	/// arguments, then answers with text once spent, so a tool loop ends.
	#[beet_core::test]
	async fn plays_its_script_then_answers() {
		let tool = || -> ToolDefinition {
			FunctionToolDefinition::new(
				"greet",
				"Greet someone",
				serde_json::json!({
					"type": "object",
					"properties": { "name": { "type": "string" } }
				}),
			)
			.unwrap()
			.into()
		};
		let mut world = (AsyncPlugin, ThreadPlugin::default()).into_world();
		let agent = world
			.spawn((Thread::default(), children![
				(Actor::user(), children![Post::spawn("go")]),
				(
					Actor::agent(),
					MockPostStreamer::with_script([
						MockTurn::call(
							"greet",
							serde_json::json!({"name":"Ada"})
						)
						.unwrap(),
						MockTurn::Text("all greeted".into()),
					]),
					children![(Router, children![tool()])],
				),
			]))
			.id();
		ThreadWindow::reduce_now(&mut world);
		let agent = world
			.entity(agent)
			.get::<Children>()
			.unwrap()
			.iter()
			.find(|child| world.entity(*child).contains::<MockPostStreamer>())
			.unwrap();
		for _ in 0..3 {
			world
				.entity_mut(agent)
				.call::<(), Outcome>(())
				.await
				.unwrap();
		}
		let thread = world.entity(agent).get::<ChildOf>().unwrap().parent();
		let posts = world.get::<ThreadWindow>(thread).unwrap().posts().to_vec();
		posts
			.iter()
			.find_map(|post| match post.as_agent_post() {
				AgentPost::FunctionCall(fc) => {
					Some((fc.name().to_string(), fc.arguments().to_string()))
				}
				_ => None,
			})
			.unwrap()
			.xpect_eq(("greet".to_string(), r#"{"name":"Ada"}"#.to_string()));
		// one call, then the text turn, then once spent the echo, never a
		// second call
		posts
			.iter()
			.filter(|post| {
				matches!(post.as_agent_post(), AgentPost::FunctionCall(_))
			})
			.count()
			.xpect_eq(1);
		posts
			.iter()
			.rev()
			.take(2)
			.map(|post| post.to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"you said: go".to_string(),
				"all greeted".to_string(),
			]);
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
		.unwrap()
		.into();

		let args = Post::run_oneshot(children![
			(Actor::user(), children![Post::spawn("place one")]),
			(Actor::agent(), MockPostStreamer::default(), children![(
				Router,
				children![tool]
			)]),
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
