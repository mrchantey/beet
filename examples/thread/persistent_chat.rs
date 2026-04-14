//!
//!
//! ```sh
//! cargo run --example persistent_chat --features=thread,bevy_scene
//! ```
use beet::prelude::*;

const SCENE_DIR: &str = "examples/thread";
const SCENE_FILE: &str = "persistent_chat.json";

fn main() {
	env_ext::load_dotenv();
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin {
				// level: Level::TRACE,
				..default()
			},
			ThreadPlugin::default(),
			ThreadStdoutPlugin::default(),
		))
		.add_systems(Startup, setup)
		.run();
}


fn setup(mut commands: Commands) {
	let bucket_path = WsPathBuf::new(SCENE_DIR).into_abs();
	let blob = FsBucket::new(bucket_path).blob(RelPath::new(SCENE_FILE));
	let clear = CliArgs::parse_env().params.contains_key("clear");

	commands.queue_async(async move |world: AsyncWorld| {
		blob.erased_bucket().bucket_try_create().await?;
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

fn default_scene(blob: TypedBlob<FsBucket>) -> impl Bundle {
	(
		Repeat::new(),
		CallOnSpawn::<(), Outcome>::default(),
		children![(
			Thread::default(),
			blob,
			Sequence::new().allow_no_action(),
			children![
				(Actor::system(), children![Post::spawn(
					r#"Ask a single, brief interesting question, followup with more brief questions based on the users' answers"#
				)]),
				(
					Actor::new("Agent", ActorKind::Agent),
					SkipIfLatest::new(OpenAiProvider::gpt_5_mini().unwrap()),
					// OllamaProvider::default_12gb()
				),
				(
					Actor::new("User", ActorKind::User),
					SkipIfLatest::new(StdinPost)
				),
			]
		)],
	)
}
