// Test cases for validating PostStreamer implementations using the
// ThreadMut/ActorViewMut fluent builder API.
use beet_core::prelude::*;
use beet_thread::prelude::*;

/// Send a simple prompt and verify we get a non-empty display post back.
pub async fn basic_text_response(
	streamer: impl Component + PostStreamer + Clone,
) {
	let mut thread = ThreadMut::new();
	thread.insert_actor(Actor::human()).insert_post(
		"Say hello in exactly 3 words. Do not include any punctuation.",
	);
	let mut actor = thread.insert_actor(Actor::agent());
	actor.with_bundle(streamer);
	let posts = actor.send_and_collect().await.unwrap();

	posts.is_empty().xpect_false();
	posts
		.iter()
		.any(|post| post.intent().is_display() && !post.to_string().is_empty())
		.xpect_true();
}

/// Same as basic but with streaming enabled. Verify we get posts back.
pub async fn streaming_response(
	streamer: impl Component + PostStreamer + Clone,
) {
	let mut thread = ThreadMut::new();
	thread
		.insert_actor(Actor::human())
		.insert_post("Count from 1 to 5.");
	let mut actor = thread.insert_actor(Actor::agent());
	actor.with_bundle(streamer);
	let posts = actor.send_and_collect().await.unwrap();

	posts.is_empty().xpect_false();
	posts
		.iter()
		.any(|post| post.intent().is_display())
		.xpect_true();
}

/// Use a system prompt and verify the response follows it.
pub async fn system_prompt(streamer: impl Component + PostStreamer + Clone) {
	let mut thread = ThreadMut::new();
	thread.insert_actor(Actor::system()).insert_post(
		"You are a pirate. Always respond in pirate speak, beginning with 'ahoy'.",
	);
	thread
		.insert_actor(Actor::human())
		.insert_post("Say hello in exactly 5 words.");
	let mut actor = thread.insert_actor(Actor::agent());
	actor.with_bundle(streamer);
	let posts = actor.send_and_collect().await.unwrap();

	let text = posts
		.iter()
		.filter(|post| post.intent().is_display())
		.map(|post| post.to_string())
		.collect::<String>()
		.to_lowercase();
	text.xpect_contains("ahoy");
}

/// Define a function tool and verify the model produces a function call.
pub async fn tool_calling(streamer: impl Component + PostStreamer + Clone) {
	let tool = FunctionTool::new(
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

	let mut thread = ThreadMut::new();
	thread
		.insert_actor(Actor::human())
		.insert_post("What's the weather like in San Francisco?");
	let mut actor = thread.insert_actor(Actor::agent());
	actor.with_bundle(streamer);
	actor.with_tool(tool);
	let posts = actor.send_and_collect().await.unwrap();

	let function_calls: Vec<_> = posts
		.iter()
		.filter_map(|post| match post.as_agent_post() {
			AgentPost::FunctionCall(fc) => {
				Some((fc.name().to_string(), fc.arguments().to_string()))
			}
			_ => None,
		})
		.collect();

	function_calls.is_empty().xpect_false();

	let (name, args) = &function_calls[0];
	name.xpect_contains("get_weather");
	args.to_lowercase().xpect_contains("san francisco");
}

/// Send a 1x1 blue PNG pixel and verify the model identifies the color.
pub async fn image_input(streamer: impl Component + PostStreamer + Clone) {
	if streamer.provider_slug() == "ollama" {
		// Ollama image support is a wip
		return;
	}

	// 1x1 blue pixel PNG https://png-pixel.com/
	let image_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPj/HwADBwIAMCbHYQAAAABJRU5ErkJggg==";
	let image_bytes =
		base64::Engine::decode(&base64::prelude::BASE64_STANDARD, image_b64)
			.unwrap();

	let image_post = IntoPost::Bytes {
		media_type: MediaType::Png,
		bytes: image_bytes,
		file_stem: None,
	};

	let mut thread = ThreadMut::new();
	let mut human = thread.insert_actor(Actor::human());
	human.insert_post(
		"What color is this image? Answer in one word, either 'red' 'green' or 'blue'.",
	);
	human.insert_post(image_post);
	let thread = human.thread_view();
	let mut actor = thread.insert_actor(Actor::agent());
	actor.with_bundle(streamer);
	let posts = actor.send_and_collect().await.unwrap();

	let text = posts
		.iter()
		.filter(|post| post.intent().is_display())
		.map(|post| post.to_string())
		.collect::<String>()
		.to_lowercase();
	text.xpect_contains("blue");
}

/// Multi-turn conversation: verify the model retains context across turns.
pub async fn multi_turn_conversation(
	streamer: impl Component + PostStreamer + Clone,
) {
	let mut thread = ThreadMut::new();
	// First turn: user introduces themselves
	thread
		.insert_actor(Actor::human())
		.insert_post("My name is Alice.");
	// Simulated assistant reply (pre-existing history)
	thread.insert_actor(Actor::agent()).insert_post(
		"Hello Alice! Nice to meet you. How can I help you today?",
	);
	// Second turn: user asks the model to recall their name
	thread
		.insert_actor(Actor::human())
		.insert_post("What is my name?");
	// The real agent that will generate a response
	let mut actor = thread.insert_actor(Actor::agent());
	actor.with_bundle(streamer);
	let posts = actor.send_and_collect().await.unwrap();

	let text = posts
		.iter()
		.filter(|post| post.intent().is_display())
		.map(|post| post.to_string())
		.collect::<String>();
	text.contains("Alice").xpect_true();
}

/// Roundtrip: generate an image description, encode a known image,
/// then verify the model can describe it back.
/// This tests the full bytes->image->text pipeline.
/// Skipped for ollama which lacks vision model support.
pub async fn image_roundtrip(
	text_streamer: impl Component + PostStreamer + Clone,
	vision_streamer: impl Component + PostStreamer + Clone,
) {
	if text_streamer.provider_slug() == "ollama" {
		return;
	}

	// Step 1: ask the model to describe what a blue square should look like
	let mut thread1 = ThreadMut::new();
	thread1
		.insert_actor(Actor::human())
		.insert_post("Describe a solid blue square image in exactly 3 words.");
	let mut actor1 = thread1.insert_actor(Actor::agent());
	actor1.with_bundle(text_streamer);
	let description_posts = actor1.send_and_collect().await.unwrap();

	let description = description_posts
		.iter()
		.filter(|post| post.intent().is_display())
		.map(|post| post.to_string())
		.collect::<String>();
	description.is_empty().xpect_false();

	// Step 2: send a known blue pixel image and ask the model to identify it
	let image_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPj/HwADBwIAMCbHYQAAAABJRU5ErkJggg==";
	let image_bytes =
		base64::Engine::decode(&base64::prelude::BASE64_STANDARD, image_b64)
			.unwrap();

	let image_post = IntoPost::Bytes {
		media_type: MediaType::Png,
		bytes: image_bytes,
		file_stem: None,
	};

	let mut thread2 = ThreadMut::new();
	let mut human2 = thread2.insert_actor(Actor::human());
	human2.insert_post(
		"What color is this image? Answer with just the color name.",
	);
	human2.insert_post(image_post);
	let thread2 = human2.thread_view();
	let mut actor2 = thread2.insert_actor(Actor::agent());
	actor2.with_bundle(vision_streamer);
	let vision_posts = actor2.send_and_collect().await.unwrap();

	let color = vision_posts
		.iter()
		.filter(|post| post.intent().is_display())
		.map(|post| post.to_string())
		.collect::<String>()
		.to_lowercase();
	color.xpect_contains("blue");
}
