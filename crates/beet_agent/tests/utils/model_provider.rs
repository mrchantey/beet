use beet_agent::prelude::*;
use beet_core::exports::futures_lite::pin;
use beet_core::prelude::*;

/// Basic text response - simple user message, validates ResponseResource schema.
pub async fn basic_text_response(provider: impl ModelProvider) {
	let body = openresponses::RequestBody::new(provider.default_small_model())
		.with_input(
			"Say hello in exactly 3 words. Do not include any punctuation.",
		);

	let response = provider.send(body).await.unwrap();

	response.object.xpect_eq("response");
	response
		.status
		.xpect_eq(openresponses::response::Status::Completed);
	response
		.model
		.as_ref()
		.map(|m| m.as_str())
		.unwrap_or("")
		.xpect_starts_with(provider.default_small_model());
	response.first_text().is_some().xpect_true();
	response.usage.is_some().xpect_true();
}

/// Streaming response - validates SSE streaming events and final response.
pub async fn streaming_response(provider: impl ModelProvider) {
	let body = openresponses::RequestBody::new(provider.default_small_model())
		.with_input("Count from 1 to 5.")
		.with_stream(true);
	let stream = provider.stream(body).await.unwrap();
	pin!(stream);

	let mut event_types = Vec::new();
	let mut final_response: Option<openresponses::ResponseBody> = None;
	let mut accumulated_text = String::new();

	while let Some(result) = stream.next().await {
		let event = result.unwrap();
		event_types.push(event.event_type().to_string());

		match event {
			openresponses::StreamingEvent::OutputTextDelta(ev) => {
				accumulated_text.push_str(&ev.delta);
			}
			openresponses::StreamingEvent::ResponseCompleted(ev) => {
				final_response = Some(ev.response);
			}
			_ => {}
		}
	}

	// Verify we received expected event types
	event_types
		.contains(&"response.created".to_string())
		.xpect_true();
	event_types
		.contains(&"response.completed".to_string())
		.xpect_true();

	// Verify we accumulated text via deltas
	accumulated_text.is_empty().xpect_false();

	// Verify final response is valid
	let response = final_response.unwrap();
	response
		.status
		.xpect_eq(openresponses::response::Status::Completed);
	response.first_text().is_some().xpect_true();
}

/// System prompt - include system role message in input.
pub async fn system_prompt(provider: impl ModelProvider) {
	let body = openresponses::RequestBody::new(provider.default_small_model())
		.with_input_items(vec![
			openresponses::request::InputItem::Message(
				openresponses::request::MessageParam::system(
					"You are a pirate. Always respond in pirate speak, begining with 'ahoy'.",
				),
			),
			openresponses::request::InputItem::Message(
				openresponses::request::MessageParam::user(
					"Say hello in exactly 5 words.",
				),
			),
		]);

	let response = provider.send(body).await.unwrap();

	response
		.status
		.xpect_eq(openresponses::response::Status::Completed);
	response.all_text().to_lowercase().xpect_contains("ahoy");
}

/// Tool calling - define a function tool and verify function_call output.
pub async fn tool_calling(provider: impl ModelProvider) {
	let tool = openresponses::FunctionToolParam::new("get_weather")
		.with_description("Get the current weather for a location")
		.with_parameters(serde_json::json!({
			"type": "object",
			"properties": {
				"location": {
					"type": "string",
					"description": "The city and state, e.g. San Francisco, CA"
				}
			},
			"required": ["location"]
		}));

	let body = openresponses::RequestBody::new(provider.default_small_model())
		.with_input("What's the weather like in San Francisco?")
		.with_tool(tool);

	let response = provider.send(body).await.unwrap();

	response
		.status
		.xpect_eq(openresponses::response::Status::Completed);

	// Verify we got a function call
	let function_calls = response.function_calls();
	function_calls.is_empty().xpect_false();

	let fc = function_calls[0];
	fc.name.xpect_eq("get_weather");
	// Status may be omitted by some providers
	if let Some(status) = fc.status {
		status.xpect_eq(openresponses::FunctionCallStatus::Completed);
	}

	// Parse and verify arguments
	let args = fc.arguments_value().unwrap();
	args["location"]
		.as_str()
		.unwrap()
		.to_lowercase()
		.contains("san francisco")
		.xpect_true();
}

/// Image input - send image URL in user content.
pub async fn image_input(provider: impl ModelProvider) {
	if provider.provider_slug() == "ollama" {
		// Ollama image support is a wip
		return;
	}

	// A simple 1x1 blue pixel PNG as base64 https://png-pixel.com/
	let image_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPj/HwADBwIAMCbHYQAAAABJRU5ErkJggg==";

	let msg = openresponses::request::MessageParam::with_parts(
		openresponses::MessageRole::User,
		vec![
			openresponses::ContentPart::input_text(
				"What color is this image? Answer in one word, either 'red' 'green' or 'blue'",
			),
			openresponses::ContentPart::InputImage(
				openresponses::InputImage::from_base64("image/png", image_b64),
			),
		],
	);

	let body = openresponses::RequestBody::new(provider.default_small_model())
		.with_input_items(vec![openresponses::request::InputItem::Message(
			msg,
		)]);

	let response = provider.send(body).await.unwrap();

	response
		.status
		.xpect_eq(openresponses::response::Status::Completed);

	let text = response.first_text().unwrap().to_lowercase();
	// The image is a blue pixel
	text.xpect_contains("blue");
}

/// Multi-turn conversation - send assistant + user messages as conversation history.
pub async fn multi_turn_conversation(provider: impl ModelProvider) {
	let body = openresponses::RequestBody::new(provider.default_small_model())
		.with_input_items(vec![
			openresponses::request::InputItem::Message(
				openresponses::request::MessageParam::user("My name is Alice."),
			),
			openresponses::request::InputItem::Message(
				openresponses::request::MessageParam::assistant(
					"Hello Alice! Nice to meet you. How can I help you today?",
				),
			),
			openresponses::request::InputItem::Message(
				openresponses::request::MessageParam::user("What is my name?"),
			),
		]);

	let response = provider.send(body).await.unwrap();
	response
		.status
		.xpect_eq(openresponses::response::Status::Completed);

	let text = response.first_text().unwrap();
	// The model should remember the name from conversation history
	text.contains("Alice").xpect_true();
}
