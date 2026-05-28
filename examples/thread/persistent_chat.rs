//! Demostrates persisting chat to the filesystem
//!
//! ```sh
//! cargo run --example persistent_chat --features=thread,world_serde
//! cargo run --example persistent_chat --features=thread,world_serde -- --new
//! ```
use beet::prelude::*;

const WORLD_SERDE_FILE: &str = "examples/thread/persistent_chat.json";

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


fn setup(async_commands: AsyncCommands) {
	cfg_if! {
		if #[cfg(feature="aws_sdk")]{
			// swap out for s3 storage by changing the store
			// see infra examples for configuring stores
			let store = S3Store::new("some-bucket","some-region");
		}else{
			let store = FsStore::new(WsPathBuf::default());
		}
	}


	let blob = store.blob(RelPath::new(WORLD_SERDE_FILE));
	let new_thread = CliArgs::parse_env().params.contains_key("new");

	async_commands.run(async move |world: AsyncWorld| {
		if new_thread {
			blob.remove().await.ok();
		}
		WorldSerdeStore::load_or_create(world, blob, async |_| {
			chat_bundle().xok()
		})
		.await?;
		Ok(())
	});
}

fn chat_bundle() -> impl Bundle {
	(
		Thread::default(),
		// adding a blob to a thread indicates it should be persisted
		Repeat::new(),
		// this control flow will be triggered on spawn,
		// including after reload
		CallOnSpawn::<(), Outcome>::default(),
		children![(
			Sequence::new(),
			// the system actor is static and has no action,
			// so the sequence will skip over it
			ExcludeErrors(ChildError::NO_ACTION),
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
