//! Mock model provider for testing.
//!
//! [`MockModelProvider`] simulates an AI model for testing purposes:
//! - If the request contains tools, it calls the first tool with default argument values
//! - If no tools are present, it echoes the input prefixed with "you said:"

use crate::openresponses;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::tasks::BoxedFuture;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

/// Counter for generating unique IDs
static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_id(prefix: &str) -> String {
	format!("{}_{}", prefix, ID_COUNTER.fetch_add(1, Ordering::SeqCst))
}

/// A mock model provider for testing tool-calling workflows.
///
/// ## Behavior
///
/// - **With tools**: Calls the first tool with default values generated from
///   the parameter schema (strings become "", integers become 0, etc.)
/// - **Without tools**: Returns the user's input prefixed with "you said:"
///
/// ## Example
///
/// ```ignore
/// use beet_clanker::prelude::*;
///
/// let provider = MockModelProvider::default();
///
/// // Without tools - echoes input
/// let body = openresponses::RequestBody::new("mock")
///     .with_input("Hello!");
/// let response = provider.send(body).await.unwrap();
/// assert!(response.first_text().unwrap().contains("you said:"));
///
/// // With tools - calls the first tool
/// let tool = openresponses::FunctionToolParam::new("greet")
///     .with_parameters(serde_json::json!({
///         "type": "object",
///         "properties": { "name": { "type": "string" } }
///     }));
/// let body = openresponses::RequestBody::new("mock")
///     .with_input("Say hi to Bob")
///     .with_tool(tool);
/// let response = provider.send(body).await.unwrap();
/// assert!(!response.function_calls().is_empty());
/// ```
#[derive(Debug, Clone, Default)]
pub struct MockModelProvider {
	/// Optional custom response text (overrides default echo behavior)
	pub custom_response: Option<String>,
}

impl MockModelProvider {
	/// Creates a new mock provider.
	pub fn new() -> Self { Self::default() }

	/// Creates a mock provider that always returns the specified text.
	pub fn with_response(text: impl Into<String>) -> Self {
		Self {
			custom_response: Some(text.into()),
		}
	}

	/// Extracts the user's input text from the request.
	fn extract_input_text(request: &openresponses::RequestBody) -> String {
		match &request.input {
			openresponses::request::Input::Text(text) => text.clone(),
			openresponses::request::Input::Items(items) => {
				for item in items {
					if let openresponses::request::InputItem::Message(msg) =
						item
					{
						if msg.role == openresponses::MessageRole::User {
							match &msg.content {
								openresponses::request::MessageContent::Text(
									text,
								) => return text.clone(),
								openresponses::request::MessageContent::Parts(
									parts,
								) => {
									for part in parts {
										if let Some(text) = part.as_text() {
											return text.to_string();
										}
									}
								}
							}
						}
					}
				}
				String::new()
			}
		}
	}

	/// Generates default arguments for a tool based on its parameter schema.
	fn generate_default_arguments(
		parameters: Option<&serde_json::Value>,
	) -> String {
		let Some(params) = parameters else {
			return "{}".to_string();
		};

		let Some(properties) =
			params.get("properties").and_then(|p| p.as_object())
		else {
			return "{}".to_string();
		};

		let mut args = serde_json::Map::new();

		for (name, schema) in properties {
			let default_value = Self::default_value_for_schema(schema);
			args.insert(name.clone(), default_value);
		}

		serde_json::to_string(&serde_json::Value::Object(args))
			.unwrap_or_else(|_| "{}".to_string())
	}

	/// Generates a default value based on JSON Schema type.
	fn default_value_for_schema(
		schema: &serde_json::Value,
	) -> serde_json::Value {
		let type_str = schema
			.get("type")
			.and_then(|t| t.as_str())
			.unwrap_or("string");

		match type_str {
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

	/// Creates a response with a text message.
	fn create_text_response(text: &str) -> openresponses::response::Body {
		openresponses::response::Body {
			id: next_id("mock-resp"),
			object: "response".to_string(),
			created_at: Some(0),
			completed_at: Some(0),
			status: openresponses::response::Status::Completed,
			error: None,
			incomplete_details: None,
			instructions: None,
			model: Some("mock".to_string()),
			output: vec![openresponses::OutputItem::Message(
				openresponses::Message {
					id: next_id("msg"),
					role: openresponses::MessageRole::Assistant,
					status: openresponses::MessageStatus::Completed,
					content: vec![openresponses::OutputContent::OutputText(
						openresponses::OutputText {
							text: text.to_string(),
							annotations: vec![],
							logprobs: vec![],
						},
					)],
				},
			)],
			parallel_tool_calls: None,
			previous_response_id: None,
			reasoning: None,
			temperature: None,
			text: None,
			top_p: None,
			truncation: None,
			usage: Some(openresponses::Usage {
				input_tokens: 10,
				output_tokens: 10,
				total_tokens: 20,
				input_tokens_details: None,
				output_tokens_details: None,
			}),
			metadata: None,
			max_output_tokens: None,
			tools: vec![],
			tool_choice: None,
			presence_penalty: None,
			frequency_penalty: None,
			top_logprobs: None,
			max_tool_calls: None,
			store: None,
			background: None,
			service_tier: None,
			safety_identifier: None,
			prompt_cache_key: None,
		}
	}

	/// Creates a response with a function call.
	fn create_function_call_response(
		name: &str,
		arguments: &str,
	) -> openresponses::response::Body {
		let call_id = next_id("call");
		openresponses::response::Body {
			id: next_id("mock-resp"),
			object: "response".to_string(),
			created_at: Some(0),
			completed_at: Some(0),
			status: openresponses::response::Status::Completed,
			error: None,
			incomplete_details: None,
			instructions: None,
			model: Some("mock".to_string()),
			output: vec![openresponses::OutputItem::FunctionCall(
				openresponses::FunctionCall {
					id: next_id("fc"),
					call_id,
					name: name.to_string(),
					arguments: arguments.to_string(),
					status: Some(openresponses::FunctionCallStatus::Completed),
				},
			)],
			parallel_tool_calls: None,
			previous_response_id: None,
			reasoning: None,
			temperature: None,
			text: None,
			top_p: None,
			truncation: None,
			usage: Some(openresponses::Usage {
				input_tokens: 10,
				output_tokens: 10,
				total_tokens: 20,
				input_tokens_details: None,
				output_tokens_details: None,
			}),
			metadata: None,
			max_output_tokens: None,
			tools: vec![],
			tool_choice: None,
			presence_penalty: None,
			frequency_penalty: None,
			top_logprobs: None,
			max_tool_calls: None,
			store: None,
			background: None,
			service_tier: None,
			safety_identifier: None,
			prompt_cache_key: None,
		}
	}
}

impl ModelProvider for MockModelProvider {
	fn provider_slug(&self) -> &'static str { "mock" }

	fn default_small_model(&self) -> &'static str { "mock-small" }

	fn default_tool_model(&self) -> &'static str { "mock-tool" }

	fn default_large_model(&self) -> &'static str { "mock-large" }

	fn send(
		&self,
		request: openresponses::RequestBody,
	) -> BoxedFuture<'_, Result<openresponses::ResponseBody>> {
		let custom_response = self.custom_response.clone();

		Box::pin(async move {
			// Check if we have tools
			if let Some(tools) = &request.tools {
				if let Some(first_tool) = tools.first() {
					// Call the first tool with default arguments
					let arguments = Self::generate_default_arguments(
						first_tool.parameters.as_ref(),
					);
					return Self::create_function_call_response(
						&first_tool.name,
						&arguments,
					)
					.xok();
				}
			}

			// No tools - echo the input or use custom response
			let response_text = if let Some(custom) = custom_response {
				custom
			} else {
				let input = Self::extract_input_text(&request);
				format!("you said: {}", input)
			};

			Self::create_text_response(&response_text).xok()
		})
	}

	fn stream(
		&self,
		request: openresponses::RequestBody,
	) -> BoxedFuture<'_, Result<StreamingEventStream>> {
		let custom_response = self.custom_response.clone();

		Box::pin(async move {
			use beet_core::exports::async_channel;

			// For streaming, we'll emit a few events to simulate the stream
			let (sender, receiver) = async_channel::bounded(10);

			// Determine what to respond with
			let (is_tool_call, response) = if let Some(tools) = &request.tools {
				if let Some(first_tool) = tools.first() {
					let arguments = Self::generate_default_arguments(
						first_tool.parameters.as_ref(),
					);
					(
						true,
						Self::create_function_call_response(
							&first_tool.name,
							&arguments,
						),
					)
				} else {
					let input = Self::extract_input_text(&request);
					let text = custom_response
						.unwrap_or_else(|| format!("you said: {}", input));
					(false, Self::create_text_response(&text))
				}
			} else {
				let input = Self::extract_input_text(&request);
				let text = custom_response
					.unwrap_or_else(|| format!("you said: {}", input));
				(false, Self::create_text_response(&text))
			};

			// Spawn a task to send events
			let response_id = response.id.clone();
			bevy::tasks::IoTaskPool::get()
				.spawn(async move {
					// Send response.created
					let _ = sender
						.send(Ok(
							openresponses::StreamingEvent::ResponseCreated(
								openresponses::streaming::ResponseCreatedEvent {
									sequence_number: 0,
									response: response.clone(),
								},
							),
						))
						.await;

					if is_tool_call {
						// For tool calls, send output item added
						if let Some(item) = response.output.first() {
							let _ = sender
								.send(Ok(
									openresponses::StreamingEvent::OutputItemAdded(
										openresponses::streaming::OutputItemAddedEvent {
											sequence_number: 1,
											output_index: 0,
											item: Some(item.clone()),
										},
									),
								))
								.await;
						}
					} else {
						// For text responses, send text deltas
						if let Some(openresponses::OutputItem::Message(msg)) =
							response.output.first()
						{
							if let Some(text) = msg.first_text() {
								let msg_id = msg.id.clone();
								// Send in chunks to simulate streaming
								let chunk_size = 10.max(text.len() / 3);
								for (idx, chunk) in text
									.as_bytes()
									.chunks(chunk_size)
									.enumerate()
								{
									let chunk_str =
										String::from_utf8_lossy(chunk)
											.to_string();
									let _ = sender
										.send(Ok(
											openresponses::StreamingEvent::OutputTextDelta(
												openresponses::streaming::OutputTextDeltaEvent {
													sequence_number: (idx + 1) as i64,
													item_id: msg_id.clone(),
													output_index: 0,
													content_index: 0,
													delta: chunk_str,
													logprobs: vec![],
													obfuscation: None,
												},
											),
										))
										.await;
								}
							}
						}
					}

					// Send response.completed
					let _ = sender
						.send(Ok(
							openresponses::StreamingEvent::ResponseCompleted(
								openresponses::streaming::ResponseCompletedEvent {
									sequence_number: 100,
									response: openresponses::response::Body {
										id: response_id,
										..response
									},
								},
							),
						))
						.await;
				})
				.detach();

			let stream = futures::stream::unfold(receiver, |rx| async move {
				match rx.recv().await {
					Ok(event) => Some((event, rx)),
					Err(_) => None,
				}
			});

			Ok(Box::pin(stream) as StreamingEventStream)
		})
	}
}


#[cfg(test)]
mod test {
	use super::*;
	use beet_core::exports::futures_lite::pin;

	#[beet_core::test]
	async fn echoes_input_without_tools() {
		let provider = MockModelProvider::default();
		let body =
			openresponses::RequestBody::new("mock").with_input("Hello world!");

		let response = provider.send(body).await.unwrap();

		response
			.status
			.xpect_eq(openresponses::response::Status::Completed);
		let text = response.first_text().unwrap();
		text.xpect_eq("you said: Hello world!");
	}

	#[beet_core::test]
	async fn calls_first_tool_when_present() {
		let provider = MockModelProvider::default();
		let tool = openresponses::FunctionToolParam::new("greet")
			.with_parameters(serde_json::json!({
				"type": "object",
				"properties": {
					"name": { "type": "string" },
					"age": { "type": "integer" }
				}
			}));

		let body = openresponses::RequestBody::new("mock")
			.with_input("Greet someone")
			.with_tool(tool);

		let response = provider.send(body).await.unwrap();

		let function_calls = response.function_calls();
		function_calls.len().xpect_eq(1);
		function_calls[0].name.xpect_eq("greet");

		// Verify default arguments were generated
		let args: serde_json::Value =
			serde_json::from_str(&function_calls[0].arguments).unwrap();
		args["name"].as_str().unwrap().xpect_eq("");
		args["age"].as_i64().unwrap().xpect_eq(0);
	}

	#[beet_core::test]
	async fn custom_response_overrides_echo() {
		let provider = MockModelProvider::with_response("Custom answer");
		let body = openresponses::RequestBody::new("mock").with_input("Hello!");

		let response = provider.send(body).await.unwrap();
		response.first_text().unwrap().xpect_eq("Custom answer");
	}

	#[beet_core::test]
	async fn streaming_echoes_input() {
		let provider = MockModelProvider::default();
		let body = openresponses::RequestBody::new("mock")
			.with_input("Stream test")
			.with_stream(true);

		let stream = provider.stream(body).await.unwrap();
		pin!(stream);

		let mut completed = false;
		let mut accumulated = String::new();

		while let Some(result) = stream.next().await {
			let event = result.unwrap();
			match event {
				openresponses::StreamingEvent::OutputTextDelta(ev) => {
					accumulated.push_str(&ev.delta);
				}
				openresponses::StreamingEvent::ResponseCompleted(_) => {
					completed = true;
				}
				_ => {}
			}
		}

		completed.xpect_true();
		accumulated.xpect_eq("you said: Stream test");
	}

	#[beet_core::test]
	async fn streaming_calls_tool() {
		let provider = MockModelProvider::default();
		let tool = openresponses::FunctionToolParam::new("test_tool")
			.with_parameters(serde_json::json!({
				"type": "object",
				"properties": { "value": { "type": "boolean" } }
			}));

		let body = openresponses::RequestBody::new("mock")
			.with_input("Call tool")
			.with_tool(tool)
			.with_stream(true);

		let stream = provider.stream(body).await.unwrap();
		pin!(stream);

		let mut found_tool_call = false;

		while let Some(result) = stream.next().await {
			let event = result.unwrap();
			if let openresponses::StreamingEvent::OutputItemAdded(ev) = event {
				if let Some(openresponses::OutputItem::FunctionCall(fc)) =
					ev.item
				{
					fc.name.xpect_eq("test_tool");
					found_tool_call = true;
				}
			}
		}

		found_tool_call.xpect_true();
	}
}
