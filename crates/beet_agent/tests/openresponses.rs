//! Integration tests for the OpenResponses API.
//!
//! These tests validate compliance with the OpenResponses specification
//! by making real API calls.
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]

use beet_agent::prelude::*;
use beet_core::prelude::*;

fn text_provider() -> impl ModelProvider {
	dotenv::dotenv().ok();
	OllamaProvider::default()
}


/// Basic text response - simple user message, validates ResponseResource schema.
#[beet_core::test]
async fn basic_text_response() {
	let mut provider = text_provider();

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
		.as_str()
		.xpect_starts_with(provider.default_small_model());
	response.first_text().is_some().xpect_true();
	response.usage.is_some().xpect_true();
}

/// Streaming response - validates SSE streaming events and final response.
#[beet_core::test]
// #[ignore]
async fn streaming_response() {
	let mut provider = text_provider();

	let body = openresponses::RequestBody::new(provider.default_small_model())
		.with_input("Count from 1 to 5.")
		.with_stream(true);
	let mut stream = provider.stream(body).await.unwrap();

	let mut events = Vec::new();
	let mut final_response: Option<openresponses::ResponseBody> = None;

	while let Some(ev) = stream.next().await {
		let ev = ev.unwrap();
		if ev.data == "[DONE]" {
			break;
		}

		let json: serde_json::Value = serde_json::from_str(&ev.data).unwrap();
		let event_type = json["type"].as_str().unwrap_or("").to_string();
		events.push(event_type.clone());

		// Capture the final response.completed event
		if event_type == "response.completed" {
			final_response =
				serde_json::from_value(json["response"].clone()).ok();
		}
	}

	// Verify we received expected event types
	events
		.contains(&"response.created".to_string())
		.xpect_true();
	events
		.contains(&"response.completed".to_string())
		.xpect_true();

	// Verify final response is valid
	let response = final_response.unwrap();
	response
		.status
		.xpect_eq(openresponses::response::Status::Completed);
	response.first_text().is_some().xpect_true();
}

/// System prompt - include system role message in input.
#[beet_core::test]
async fn system_prompt() {
	let mut provider = text_provider();

	let body = openresponses::RequestBody::new(provider.default_small_model())
		.with_input_items(vec![
			openresponses::request::InputItem::Message(
				openresponses::request::MessageParam::system(
					"You are a pirate. Always respond in pirate speak.",
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
	let text = response.first_text().unwrap().to_lowercase();
	// Should contain pirate-y language
	(text.contains("ahoy")
		|| text.contains("matey")
		|| text.contains("arr")
		|| text.contains("ye"))
	.xpect_true();
}

/// Tool calling - define a function tool and verify function_call output.
#[beet_core::test]
async fn tool_calling() {
	let mut provider = text_provider();

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
	fc.status
		.xpect_eq(openresponses::FunctionCallStatus::Completed);

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
#[beet_core::test]
async fn image_input() {
	let mut provider = text_provider();

	// A simple 1x1 green pixel PNG as base64
	let image_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

	let msg = openresponses::request::MessageParam::with_parts(
		openresponses::MessageRole::User,
		vec![
			openresponses::ContentPart::input_text(
				"What color is this image? Answer in one word.",
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
	// The image is a green pixel
	text.contains("green").xpect_true();
}

/// Multi-turn conversation - send assistant + user messages as conversation history.
#[beet_core::test]
async fn multi_turn_conversation() {
	let mut provider = text_provider();

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
