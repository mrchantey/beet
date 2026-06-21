// Test cases for validating PostStreamer implementations via the declarative
// `run_oneshot` helper: a thread is authored as an author scene of actors with
// seed posts, the agent runs, and its reply is collected.
use beet_core::prelude::*;
use beet_thread::prelude::*;

/// Send a simple prompt and verify we get a non-empty display post back.
pub async fn basic_text_response(
	streamer: impl Component + PostStreamer + Clone,
) {
	run_oneshot(children![
		(Actor::user(), children![Post::spawn(
			"complete the sequence: one, two, three, ____"
		)]),
		(Actor::agent(), streamer),
	])
	.await
	.unwrap()
	.into_iter()
	.find(|post| post.intent().is_display())
	.unwrap()
	.to_string()
	.xpect_contains("four");
}

/// Same as basic but with streaming enabled. Verify we get posts back.
pub async fn streaming_response(
	streamer: impl Component + PostStreamer + Clone,
) {
	run_oneshot(children![
		(Actor::user(), children![Post::spawn("Count from 1 to 5.")]),
		(Actor::agent(), streamer),
	])
	.await
	.unwrap()
	.into_iter()
	.find(|post| post.intent().is_display())
	.unwrap()
	.to_string()
	.is_empty()
	.xpect_false();
}

/// Use a system prompt and verify the response follows it.
pub async fn system_prompt(streamer: impl Component + PostStreamer + Clone) {
	run_oneshot(children![
		(Actor::system(), children![Post::spawn(
			"You are a pirate. Always respond in pirate speak, beginning with 'ahoy'."
		)]),
		(Actor::user(), children![Post::spawn(
			"Say hello in exactly 5 words."
		)]),
		(Actor::agent(), streamer),
	])
	.await
	.unwrap()
	.into_iter()
	.filter(|post| post.intent().is_display())
	.map(|post| post.to_string())
	.collect::<String>()
	.to_lowercase()
	.xpect_contains("ahoy");
}

/// Define a function tool and verify the model produces a function call.
pub async fn tool_calling(streamer: impl Component + PostStreamer + Clone) {
	let tool: ToolDefinition = FunctionToolDefinition::new(
		"get_weather",
		"Get the current weather for a location",
		serde_json::json!({
			"type": "object",
			"properties": {
				"location": {
					"type": "string",
					"description": "The city and state, e.g. San Francisco, CA"
				}
			},
			"required": ["location"]
		}),
	)
	.into();

	let (name, args) = run_oneshot(children![
		(Actor::user(), children![Post::spawn(
			"What's the weather like in San Francisco?"
		)]),
		(Actor::agent(), streamer, children![tool]),
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

	name.xpect_contains("get_weather");
	args.to_lowercase().xpect_contains("san francisco");
}

/// Send a 1x1 blue PNG pixel and verify the model identifies the color.
pub async fn image_input(streamer: impl Component + PostStreamer + Clone) {
	if streamer.provider_slug() == "ollama" {
		return;
	}

	// 1x1 blue pixel PNG https://png-pixel.com/
	let image_bytes = base64::Engine::decode(
		&base64::prelude::BASE64_STANDARD,
		"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPj/HwADBwIAMCbHYQAAAABJRU5ErkJggg==",
	)
	.unwrap();

	run_oneshot(children![
		(
			Actor::user(),
			children![
				Post::spawn(
					"What color is this image? Answer in one word, either 'red' 'green' or 'blue'."
				),
				Post::spawn(IntoPost::Bytes {
					media_type: MediaType::Png,
					bytes: image_bytes,
					file_stem: None,
				}),
			]
		),
		(Actor::agent(), streamer),
	])
	.await
	.unwrap()
	.into_iter()
	.filter(|post| post.intent().is_display())
	.map(|post| post.to_string())
	.collect::<String>()
	.to_lowercase()
	.xpect_contains("blue");
}

/// Multi-turn conversation: verify the model retains context across turns.
pub async fn multi_turn_conversation(
	streamer: impl Component + PostStreamer + Clone,
) {
	run_oneshot(children![
		(Actor::user(), children![Post::spawn("My name is Alice.")]),
		(Actor::agent(), children![Post::spawn(
			"Hello Alice! Nice to meet you. How can I help you today?"
		)]),
		(Actor::user(), children![Post::spawn("What is my name?")]),
		(Actor::agent(), streamer),
	])
	.await
	.unwrap()
	.into_iter()
	.filter(|post| post.intent().is_display())
	.map(|post| post.to_string())
	.collect::<String>()
	.xpect_contains("Alice");
}

/// Image roundtrip: generate an image, then interpret it.
/// 1. User asks agent to create a blue image.
/// 2. Agent generates a blue image (or fallback to known blue pixel).
/// 3. User asks agent to interpret the image.
pub async fn image_roundtrip(streamer: impl Component + PostStreamer + Clone) {
	if streamer.provider_slug() == "ollama" {
		return;
	}

	// Step 1: ask the agent to generate a blue image
	let posts = run_oneshot(children![
		(Actor::user(), children![Post::spawn(
			"Create a solid blue 1x1 pixel image. Return only the image."
		)]),
		(Actor::agent(), streamer.clone()),
	])
	.await
	.unwrap();

	// Step 2: extract the generated image, falling back to a known blue pixel
	// if the model or pipeline doesn't support image generation responses yet
	let image_post = posts
		.iter()
		.find(|post| post.media_type().is_image())
		.map(|post| IntoPost::Bytes {
			media_type: post.media_type().clone(),
			bytes: post.body_bytes().to_vec(),
			file_stem: None,
		})
		.unwrap_or_else(|| IntoPost::Bytes {
			media_type: MediaType::Png,
			bytes: base64::Engine::decode(
				&base64::prelude::BASE64_STANDARD,
				"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPj/HwADBwIAMCbHYQAAAABJRU5ErkJggg==",
			)
			.unwrap(),
			file_stem: None,
		});

	// Step 3: ask the agent to interpret the image
	run_oneshot(children![
		(
			Actor::user(),
			children![
				Post::spawn(
					"What color is this image? Answer in one word, either 'red' 'green' or 'blue'."
				),
				Post::spawn(image_post),
			]
		),
		(Actor::agent(), streamer),
	])
	.await
	.unwrap()
	.into_iter()
	.find(|post| post.intent().is_display())
	.unwrap()
	.to_string()
	.to_lowercase()
	.xpect_contains("blue");
}
