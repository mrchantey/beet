//! Demostrates persisting chat to the filesystem
//!
//! ```sh
//! cargo run --example persistent_chat --features=thread,bevy_scene
//! cargo run --example persistent_chat --features=thread,bevy_scene -- --new
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


	let blob = bucket.blob(RelPath::new(SCENE_FILE));
	let new_thread = CliArgs::parse_env().params.contains_key("new");

	commands.queue_async(async move |world: AsyncWorld| {
		if new_thread {
			blob.remove().await.ok();
		}
		let store = world
			.spawn_then((blob.clone(), SceneStore::default()))
			.await;
		SceneStore::load_or_create(store, async |_| scene().xok()).await?;
		Ok(())
	});
}

fn scene() -> impl Bundle {
	(
		Thread::default(),
		// adding a blob to a thread indicates it should be persisted
		Repeat::new(),
		// this control flow will be triggered on spawn,
		// including after scene reload
		CallOnSpawn::<(), Outcome>::default(),
		children![(
			Sequence::new()
				// the system actor is static and has no action,
				// so the sequence will skip over it
				.allow_no_action(),
			children![
				(Actor::system(), children![Post::spawn(
					r#"Ask a single brief, challenging question about the user's life choices. Followup with more brief questions based on the users' answers"#
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
