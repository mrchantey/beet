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
			ThreadPlugin::default(),
			ThreadStdoutPlugin::default(),
			BucketPlugin,
		))
		.register_type::<SaveScene>()
		.add_systems(Startup, setup)
		.run();
}

#[derive(Clone)]
struct PersistentThread {
	blob: Blob,
	spawn: OnSpawnClone,
}

impl PersistentThread {
	pub fn new(blob: Blob, func: impl CloneEntityFunc) -> Self {
		Self {
			blob,
			spawn: OnSpawnClone::new(func),
		}
	}
}

fn setup(mut commands: Commands) {
	let bucket_path = WsPathBuf::new(SCENE_DIR).into_abs();
	let bucket = FsBucketProvider::new(bucket_path);
	let route = RelPath::new(SCENE_FILE);
	let clear = CliArgs::parse_env().params.contains_key("clear");

	commands.queue_async(async move |world: AsyncWorld| {
		bucket.bucket_try_create().await?;

		if clear {
			bucket.remove(&route).await.ok();
		}

		match bucket.get(&route).await {
			Ok(scene_bytes) => {
				// Scene exists, load it and call the root
				world
					.with_then(move |world: &mut World| -> Result {
						SceneLoader::new(world).load_json(&scene_bytes)
					})
					.await?;
			}
			Err(_) => {
				// No existing scene, create fresh
				world.with(move |world: &mut World| {
					world
						.commands()
						.spawn(default_scene(bucket))
						.call::<(), Outcome>((), default());
				});
			}
		}
		Ok(())
	});
}

fn default_scene(bucket_provider: impl Component) -> impl Bundle {
	(
		bucket_provider,
		Repeat::new(),
		CallOnSpawn::<(), Outcome>::default(),
		children![(
			Thread::default(),
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
				// save directly after agent post
				SaveScene,
				(
					Actor::new("User", ActorKind::User),
					SkipIfLatest::new(StdinPost)
				),
				// save directly after user post
				SaveScene
			]
		)],
	)
}

/// On each loop, saves the scene to the bucket via [`FsBucketProvider`].
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
async fn SaveScene(cx: ActionContext) -> Result<Outcome> {
	let (bucket, json) = cx
		.caller
		.with_then(|mut entity| -> Result<_> {
			let root =
				entity.with_state::<AncestorQuery, _>(|entity, query| {
					query.root_ancestor(entity)
				});
			let world = entity.into_world_mut();
			let bucket = world.entity(root).get_or_else::<Bucket>()?.clone();
			let json =
				SceneSaver::new(world).with_entity_tree(root).save_json()?;
			(bucket, json).xok()
		})
		.await?;
	let route = RelPath::new(SCENE_FILE);
	bucket.insert(&route, json).await?;
	Ok(PASS)
}
