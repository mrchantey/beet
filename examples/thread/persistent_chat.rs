//! Demostrates persisting chat to the filesystem
//!
//! ```sh
//! cargo run --example persistent_chat --features=thread,bevy_scene
//! cargo run --example persistent_chat --features=thread,bevy_scene -- --clear
//! ```
use beet::prelude::*;

const SCENE_FILE: &str = "examples/thread/persistent_chat.json";

fn main() {
	env_ext::load_dotenv();
	App::new()
		.add_plugins((
			MinimalPlugins,
			// LogPlugin {
			// 	// level: Level::TRACE,
			// 	..default()
			// },
			ThreadPlugin::default(),
			ThreadStdoutPlugin::default(),
		))
		.add_systems(Startup, setup)
		.run();
}


fn setup(mut commands: Commands) {
	cfg_if! {
		if #[cfg(feature="aws")]{
			// swap out for s3 storage by changing the bucket
			// see infra examples for configuring buckets
			let bucket = S3Bucket::new("some-bucket","some-region");
		}else{
			let bucket = FsBucket::new(WsPathBuf::default());
		}
	}

	// create some space for the output
	println!("");

	let blob = bucket.blob(RelPath::new(SCENE_FILE));
	let clear = CliArgs::parse_env().params.contains_key("clear");

	commands.queue_async(async move |world: AsyncWorld| {
		if clear {
			blob.remove().await.ok();
		}
		thread_store::load_or_spawn(world, blob.to_blob(), move |world| {
			world.commands().spawn(default_scene(blob));
		})
		.await?;

		Ok(())
	});
}

fn default_scene(blob: impl Component) -> impl Bundle {
	(
		Repeat::new(),
		// this control flow will be triggered on spawn,
		// including after scene reload
		CallOnSpawn::<(), Outcome>::default(),
		children![(
			Thread::default(),
			// adding a blob to a thread indicates it should be persisted
			blob,
			Sequence::new()
				// the system actor is static and has no action,
				// so the sequence will skip over it
				.allow_no_action(),
			children![
				(Actor::system(), children![Post::spawn(
					r#"Ask a single brief, highly relevent and deeply contraversial question
related to the users personal life, not the mundane.
followup with more brief questions based on the users' answers
"#
				)]),
				(
					Actor::new("Agent", ActorKind::Agent),
					// if this actor was not the last to post,
					// get a post from the model
					SkipIfLatest::new(OpenAiProvider::gpt_5_mini().unwrap()),
					// OllamaProvider::default_12gb()
				),
				(
					Actor::new("User", ActorKind::User),
					// if this actor was not the last to post,
					// get a post from stdin
					SkipIfLatest::new(StdinPost)
				),
			]
		)],
	)
}
