beet_core::test_main!();

use beet_thread::prelude::*;

#[path = "utils/post_streamer.rs"]
mod post_streamer;

fn streamer() -> O11sStreamer { OllamaProvider::default_12gb() }

fn streamer_non_streaming() -> O11sStreamer {
	OllamaProvider::default_12gb().without_streaming()
}

fn completions_streamer() -> CompletionsStreamer {
	OllamaProvider::default_12gb_completions()
}

fn completions_streamer_non_streaming() -> CompletionsStreamer {
	OllamaProvider::default_12gb_completions().without_streaming()
}

// === PostStreamer (O11s) tests ===

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn basic_text_response() {
	post_streamer::basic_text_response(streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn streaming_response() {
	post_streamer::streaming_response(streamer()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn system_prompt() {
	post_streamer::system_prompt(streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn tool_calling() {
	post_streamer::tool_calling(streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn image_input() {
	post_streamer::image_input(streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn multi_turn_conversation() {
	post_streamer::multi_turn_conversation(streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn opens_with_a_note() {
	post_streamer::opens_with_a_note(streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn note_between_turns() {
	post_streamer::note_between_turns(streamer_non_streaming()).await;
}

// === PostStreamer (Completions) tests ===

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn cs_basic_text_response() {
	post_streamer::basic_text_response(completions_streamer_non_streaming())
		.await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn cs_streaming_response() {
	post_streamer::streaming_response(completions_streamer()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn cs_system_prompt() {
	post_streamer::system_prompt(completions_streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn cs_tool_calling() {
	post_streamer::tool_calling(completions_streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn cs_image_input() {
	post_streamer::image_input(completions_streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn cs_multi_turn_conversation() {
	post_streamer::multi_turn_conversation(
		completions_streamer_non_streaming(),
	)
	.await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn cs_opens_with_a_note() {
	post_streamer::opens_with_a_note(completions_streamer_non_streaming())
		.await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn cs_note_between_turns() {
	post_streamer::note_between_turns(completions_streamer_non_streaming())
		.await;
}

// === Image Roundtrip tests ===

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn image_roundtrip() {
	post_streamer::image_roundtrip(streamer_non_streaming()).await;
}

#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 60_000)]
async fn cs_image_roundtrip() {
	post_streamer::image_roundtrip(completions_streamer_non_streaming()).await;
}

/// A compactor on the local model: a fact is edited and another deleted
/// after a compaction, and the refreshed summary is printed beside the first
/// for a person to read, since what a model leaves out nothing can prove.
#[ignore = "requires Ollama running locally"]
#[beet_core::test(timeout_ms = 600_000)]
async fn compaction_refresh_on_a_local_model() {
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_scene::prelude::*;

	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.install_types::<ThreadPlugin>();
	let store = BlobStore::temp();
	store
		.insert(
			&fwd_path!("Compactor.bsx"),
			r#"<RepeatWhileFunctionCallOutput>
	<Thread {Sequence}>
		<CompactInstructions/>
		<CreateActor name="Reader" kind="User"/>
		<CreateActor name="Compactor" kind="Agent" {ModelStreamer{provider:Ollama, size:Large, effort:None}}>
			<Router><ReadPosts/></Router>
		</CreateActor>
	</Thread>
</RepeatWhileFunctionCallOutput>"#,
		)
		.await
		.unwrap();
	let root = app
		.world_mut()
		.spawn((store.clone(), JsonTableStore::over(store.clone()).unwrap()))
		.id();
	let window = BsxParser::spawn(
		app.world_mut(),
		r#"<Thread {(Sequence, MountThreadStore{path:"chat"})}>
	<CreateActor name="Ann" kind="User"/>
	<CreateActor name="Coach" kind="Agent" {(MockPostStreamer{custom_response:"noted"}, Compact{template:Document("Compactor.bsx"), at:80, keep:20})}/>
</Thread>"#,
	)
	.unwrap();
	app.world_mut().entity_mut(window).insert(ChildOf(root));
	app.world_mut()
		.run_async_local_then(async move |world: AsyncWorld| -> Result {
			Thread::enter(
				world.clone(),
				window,
				ThreadEntry::parse(Request::get("chat").parts().clone())?,
			)
			.await
		})
		.await
		.unwrap();
	let ann = ActorId::from_static("ann");
	let coach = app
		.world_mut()
		.query::<(Entity, &ActorRef)>()
		.iter(app.world())
		.find(|(_, actor)| actor.0 == ActorId::from_static("coach"))
		.unwrap()
		.0;
	let say = |app: &mut App, text: &str| -> Post {
		let mut entity = app.world_mut().entity_mut(window);
		let thread = entity.get::<Thread>().unwrap().id();
		let post = AgentPost::new_text(
			ActorId::from_static("ann"),
			thread,
			text,
			PostStatus::Completed,
		);
		entity
			.get_mut::<ThreadRecords>()
			.unwrap()
			.upsert_post(post.clone());
		post
	};
	let meeting =
		say(&mut app, "Our team meeting is on Monday at 10am in room 4.");
	say(&mut app, "My dog is called Rex and he is a border collie.");
	let locker =
		say(&mut app, "My gym locker code is 4471, please remember it.");
	say(
		&mut app,
		"I am learning Spanish and practise every evening.",
	);
	say(&mut app, "My sister Mia is visiting from Perth next month.");
	say(
		&mut app,
		"I prefer tea over coffee, green tea in particular.",
	);
	app.world_mut()
		.entity_mut(coach)
		.call::<(), Outcome>(())
		.await
		.unwrap();
	let summaries = |app: &App| {
		app.world()
			.get::<ThreadRecords>(window)
			.unwrap()
			.posts()
			.iter()
			.filter_map(|post| post.log::<Compaction>())
			.map(|body| body.text)
			.collect::<Vec<_>>()
	};
	summaries(&app).len().xpect_eq(1);
	app.world_mut()
		.run_async_local_then(async move |world: AsyncWorld| -> Result {
			EditPost::new(
				ann.clone(),
				meeting.id(),
				"Our team meeting moved to Tuesday at 3pm in room 4.",
			)
			.apply(&world.entity(window))
			.await?;
			DeletePost::new(ann, locker.id())
				.apply(&world.entity(window))
				.await
		})
		.await
		.unwrap();
	app.world_mut()
		.entity_mut(coach)
		.call::<(), Outcome>(())
		.await
		.unwrap();
	let summaries = summaries(&app);
	summaries.len().xpect_eq(2);
	format!(
		"FIRST SUMMARY:\n{}\n\nREFRESHED SUMMARY:\n{}",
		summaries[0], summaries[1]
	)
	.xprint_display();
}
