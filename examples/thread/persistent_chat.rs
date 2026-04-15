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


	let blob = bucket.blob(RelPath::new(SCENE_FILE));
	let clear = CliArgs::parse_env().params.contains_key("clear");

	// create some space for the output
	println!("");

	commands.queue_async(async move |world: AsyncWorld| {
		if !blob.exists().await? || clear {
			write_scene(world, blob.to_blob()).await?;
		}
		world.spawn((blob, SceneStore::default()));
		Ok(())
	});
}


/// Temporarily spawn the hardcoded scene to serialize it.
/// as bsn and editor tooling matures this step will be done
/// outside of the binary
///
// Note the CallOnSpawn will not be made, as the entity is removed
// before any systems run.
async fn write_scene(world: AsyncWorld, blob: Blob) -> Result {
	let media_type = blob.media_type().unwrap_or(MediaType::Json);
	let bytes = world
		.with_then(move |world| -> Result<_> {
			let scene_entity = world.spawn(default_scene()).id();
			let bytes = SceneSaver::new(world)
				.with_entity_tree(scene_entity)
				.save(media_type)?;
			world.entity_mut(scene_entity).despawn();
			bytes.xok()
		})
		.await?;
	blob.insert(bytes).await?;
	Ok(())
}


fn default_scene() -> impl Bundle {
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
