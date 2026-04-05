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
			// logs all agent messages to stdout
			ThreadStdoutPlugin::default(),
			BucketPlugin,
		))
		.register_type::<Root>()
		.register_type::<SaveScene>()
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands) {
	let bucket_path = WsPathBuf::new(SCENE_DIR).into_abs();
	let fs_provider = FsBucketProvider::new(bucket_path);
	let bucket = Bucket::new(fs_provider.clone());
	let route = RoutePath::from(format!("/{SCENE_FILE}"));
	let clear = CliArgs::parse_env().params.contains_key("clear");

	commands.queue_async(async move |world: AsyncWorld| {
		bucket.bucket_try_create().await?;

		if clear {
			bucket.remove(&route).await.ok();
		}

		match bucket.get(&route).await {
			Ok(scene_bytes) => {
				// Scene exists, load it
				world
					.with_then(move |world: &mut World| -> Result {
						SceneLoader::new(world).load_json(&scene_bytes)?;
						let root = world
							.query_filtered_once::<Entity, With<Root>>()
							.into_iter()
							.next()
							.ok_or_else(|| {
								bevyhow!("root not found in loaded scene")
							})?;
						world
							.commands()
							.entity(root)
							.call::<(), Outcome>((), default());
						Ok(())
					})
					.await?;
			}
			Err(_) => {
				// No existing scene, create fresh
				world.with(move |world: &mut World| {
					world
						.commands()
						.spawn((Root, fs_provider, Repeat::new(), children![(
							Thread::default(),
							Sequence::new().allow_no_tool(),
							children![
								(Actor::system(), children![Post::spawn(
									"Ask a single, brief interesting question,
									followup with more brief questions based on the users' answers"
								)]),
								(
									Actor::new("Agent", ActorKind::Agent),
									SkipIfLatest::<O11sStreamer>::new(),
									// OllamaProvider::qwen()
									OpenAiProvider::gpt_5_mini().unwrap()
								),
								// save directly after agent post
								SaveScene,
								(
									Actor::new("User", ActorKind::User),
									SkipIfLatest::<StdinPost>::new()
								),
								// save directly after user post
								SaveScene
							]
						)]))
						.call::<(), Outcome>((), default());
				});
			}
		}
		Ok(())
	});
}


#[derive(Component, Reflect)]
#[reflect(Component)]
struct Root;


/// On each loop, saves the scene to the bucket via [`FsBucketProvider`].
#[tool]
#[derive(Component, Reflect)]
#[reflect(Component)]
async fn SaveScene(world: AsyncToolIn) -> Result<Outcome> {
	let (bucket, json) = world
		.caller
		.world()
		.with_then(|world| -> Result<_> {
			let root = world
				.query_filtered_once::<Entity, With<Root>>()
				.into_iter()
				.next()
				.ok_or_else(|| {
					bevyhow!("Root entity not found for SaveScene")
				})?;
			let bucket = world
				.get::<Bucket>(root)
				.ok_or_else(|| bevyhow!("Bucket not found on Root entity"))?
				.clone();
			let json =
				SceneSaver::new(world).with_entity_tree(root).save_json()?;
			(bucket, json).xok()
		})
		.await?;
	let route = RoutePath::from(format!("/{SCENE_FILE}"));
	bucket.bucket_try_create().await?;
	bucket.insert(&route, json).await?;
	Ok(PASS)
}
