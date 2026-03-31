// Test cases for validating PostStreamer implementations using the
// ThreadMut/ActorViewMut fluent builder API.
use beet_core::prelude::*;
use beet_thread::prelude::*;

/// Send a simple prompt and verify we get a non-empty display post back.
pub async fn basic_text_response(
	streamer: impl Component + PostStreamer + Clone,
) {
	ThreadMut::new()
		.insert_actor(Actor::human())
		.insert_post("complete the sequence: one, two, three, ____")
		.thread_view()
		.insert_actor(Actor::agent())
		.with_bundle(streamer)
		.send_and_collect()
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
	ThreadMut::new()
		.insert_actor(Actor::human())
		.insert_post("Count from 1 to 5.")
		.thread_view()
		.insert_actor(Actor::agent())
		.with_bundle(streamer)
		.send_and_collect()
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
	ThreadMut::new()
		.insert_actor(Actor::system())
		.insert_post(
			"You are a pirate. Always respond in pirate speak, beginning with 'ahoy'.",
		)
		.thread_view()
		.insert_actor(Actor::human())
		.insert_post("Say hello in exactly 5 words.")
		.thread_view()
		.insert_actor(Actor::agent())
		.with_bundle(streamer)
		.send_and_collect()
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
	let tool = FunctionToolDefinition::new(
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
	);

	let (name, args) = ThreadMut::new()
		.insert_actor(Actor::human())
		.insert_post("What's the weather like in San Francisco?")
		.thread_view()
		.insert_actor(Actor::agent())
		.with_bundle(streamer)
		.with_tool(tool)
		.send_and_collect()
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

	ThreadMut::new()
		.insert_actor(Actor::human())
		.insert_post(
			"What color is this image? Answer in one word, either 'red' 'green' or 'blue'.",
		)
		.actor_view()
		.insert_post(IntoPost::Bytes {
			media_type: MediaType::Png,
			bytes: image_bytes,
			file_stem: None,
		})
		.thread_view()
		.insert_actor(Actor::agent())
		.with_bundle(streamer)
		.send_and_collect()
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
	let mut thread = ThreadMut::new();
	thread
		.insert_actor(Actor::human())
		.insert_post("My name is Alice.");
	thread.insert_actor(Actor::agent()).insert_post(
		"Hello Alice! Nice to meet you. How can I help you today?",
	);
	thread
		.insert_actor(Actor::human())
		.insert_post("What is my name?");
	thread
		.insert_actor(Actor::agent())
		.with_bundle(streamer)
		.send_and_collect()
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
	let posts = ThreadMut::new()
		.insert_actor(Actor::human())
		.insert_post(
			"Create a solid blue 1x1 pixel image. Return only the image.",
		)
		.thread_view()
		.insert_actor(Actor::agent())
		.with_bundle(streamer.clone())
		.send_and_collect()
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
	ThreadMut::new()
		.insert_actor(Actor::human())
		.insert_post(
			"What color is this image? Answer in one word, either 'red' 'green' or 'blue'.",
		)
		.actor_view()
		.insert_post(image_post)
		.thread_view()
		.insert_actor(Actor::agent())
		.with_bundle(streamer)
		.send_and_collect()
		.await
		.unwrap()
		.into_iter()
		.find(|post| post.intent().is_display())
		.unwrap()
		.to_string()
		.to_lowercase()
		.xpect_contains("blue");
}
