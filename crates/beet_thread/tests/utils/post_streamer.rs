// Test cases for validating PostStreamer implementations via the declarative
// `Post::run_oneshot` helper: a thread is authored as an author scene of actors with
// seed posts, the agent runs, and its reply is collected.
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::Router;
use beet_thread::prelude::*;

/// Send a simple prompt and verify we get a non-empty display post back.
pub async fn basic_text_response(
	streamer: impl Component + PostStreamer + Clone,
) {
	Post::run_oneshot(children![
		(Actor::user(), children![Post::spawn(
			"complete the sequence: one, two, three, ____"
		)]),
		(Actor::agent(), streamer),
	])
	.await
	.unwrap()
	.into_iter()
	.find(|post| matches!(post.kind(), PostKind::Text | PostKind::Refusal))
	.unwrap()
	.to_string()
	.xpect_contains("four");
}

/// Same as basic but with streaming enabled. Verify we get posts back.
pub async fn streaming_response(
	streamer: impl Component + PostStreamer + Clone,
) {
	Post::run_oneshot(children![
		(Actor::user(), children![Post::spawn("Count from 1 to 5.")]),
		(Actor::agent(), streamer),
	])
	.await
	.unwrap()
	.into_iter()
	.find(|post| matches!(post.kind(), PostKind::Text | PostKind::Refusal))
	.unwrap()
	.to_string()
	.is_empty()
	.xpect_false();
}

/// Use a system prompt and verify the response follows it.
pub async fn system_prompt(streamer: impl Component + PostStreamer + Clone) {
	Post::run_oneshot(children![
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
	.filter(|post| matches!(post.kind(), PostKind::Text | PostKind::Refusal))
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
		value!({
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
	.unwrap()
	.into();

	let (name, args) = Post::run_oneshot(children![
		(Actor::user(), children![Post::spawn(
			"What's the weather like in San Francisco?"
		)]),
		(Actor::agent(), streamer, children![(Router, children![
			tool
		])]),
	])
	.await
	.unwrap()
	.into_iter()
	.filter_map(|post| match post.as_agent_post()? {
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

	Post::run_oneshot(children![
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
	.filter(|post| matches!(post.kind(), PostKind::Text | PostKind::Refusal))
	.map(|post| post.to_string())
	.collect::<String>()
	.to_lowercase()
	.xpect_contains("blue");
}

/// Multi-turn conversation: verify the model retains context across turns.
pub async fn multi_turn_conversation(
	streamer: impl Component + PostStreamer + Clone,
) {
	reply_after_joining(streamer, |records, thread, person, agent| {
		for (author, text) in [
			(&person, "My name is Alice."),
			(
				&agent,
				"Hello Alice! Nice to meet you. How can I help you today?",
			),
			(&person, "What is my name?"),
		] {
			records.upsert_post(AgentPost::new_text(
				author.clone(),
				thread,
				text,
				PostStatus::Completed,
			));
		}
		Ok(())
	})
	.await
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
	let posts = Post::run_oneshot(children![
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
		.find_map(|post| match post.as_agent_post()? {
			AgentPost::Media(view) if view.media_type().is_image() => {
				Some(IntoPost::Bytes {
					media_type: view.media_type(),
					bytes: post.bytes()?.to_vec(),
					file_stem: None,
				})
			}
			_ => None,
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
	Post::run_oneshot(children![
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
	.find(|post| matches!(post.kind(), PostKind::Text | PostKind::Refusal))
	.unwrap()
	.to_string()
	.to_lowercase()
	.xpect_contains("blue");
}

/// A thread opening with a note, the joins of its person and its agent ahead
/// of the first post, is accepted and answered.
pub async fn opens_with_a_note(
	streamer: impl Component + PostStreamer + Clone,
) {
	reply_after_joining(streamer, |records, thread, person, _| {
		records.upsert_post(AgentPost::new_text(
			person,
			thread,
			"complete the sequence: one, two, three, ____",
			PostStatus::Completed,
		));
		Ok(())
	})
	.await
	.xpect_contains("four");
}

/// A thread holding a note between two turns, the person joining again
/// after the agent's answer, is accepted and keeps its context.
pub async fn note_between_turns(
	streamer: impl Component + PostStreamer + Clone,
) {
	reply_after_joining(streamer, |records, thread, person, agent| {
		for (author, text) in [
			(&person, "My name is Alice."),
			(&agent, "Hello Alice! Nice to meet you."),
		] {
			records.upsert_post(AgentPost::new_text(
				author.clone(),
				thread,
				text,
				PostStatus::Completed,
			));
		}
		records.join(thread, [person.clone()], &default())?;
		records.upsert_post(AgentPost::new_text(
			person,
			thread,
			"What is my name?",
			PostStatus::Completed,
		));
		Ok(())
	})
	.await
	.xpect_contains("Alice");
}

/// Enter a thread of a person and `streamer`'s agent, so both join, let
/// `script` post into its records, given the thread, the person and the
/// agent, then run the agent's turn: what it said.
async fn reply_after_joining(
	streamer: impl Component + PostStreamer + Clone,
	script: impl 'static
	+ Send
	+ FnOnce(&mut ThreadRecords, ThreadId, ActorId, ActorId) -> Result,
) -> String {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.install_types::<ThreadPlugin>();
	let (person, agent) = (Actor::user(), Actor::agent());
	let (person_id, agent_id) = (person.id().clone(), agent.id().clone());
	let thread = app
		.world_mut()
		.spawn((
			Thread::default(),
			Sequence::new(),
			BypassErrors(ChildError::NO_ACTION),
			children![person, (agent, streamer)],
		))
		.id();
	app.world_mut()
		.run_async_local_then(async move |world: AsyncWorld| -> Result {
			Thread::enter(
				world.clone(),
				thread,
				ThreadEntry::parse(Request::get("chat").parts().clone())?,
			)
			.await
		})
		.await
		.unwrap();
	let mut window = app.world_mut().entity_mut(thread);
	let id = window.get::<Thread>().unwrap().id();
	script(
		&mut window.get_mut::<ThreadRecords>().unwrap(),
		id,
		person_id,
		agent_id,
	)
	.unwrap();
	let asked = window.get::<ThreadRecords>().unwrap().posts().len();
	window.call::<(), Outcome>(()).await.unwrap();
	app.world().get::<ThreadRecords>(thread).unwrap().posts()[asked..]
		.iter()
		.filter(|post| {
			matches!(post.kind(), PostKind::Text | PostKind::Refusal)
		})
		.map(|post| post.to_string())
		.collect()
}
