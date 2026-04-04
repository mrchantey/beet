//!
//!
//! ```sh
//! cargo run --example persistent_chat --features=thread,bevy_scene
//! ```
use beet::prelude::*;

const SCENE_PATH: &str = "examples/thread/persistent_chat.json";

fn main() {
	env_ext::load_dotenv();
	App::new()
		.add_plugins((
			MinimalPlugins,
			ThreadPlugin::default(),
			// logs all agent messages to stdout
			ThreadStdoutPlugin::default(),
		))
		.register_type::<Root>()
		.register_type::<SaveScene>()
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands) {
	let asset_path = WsPathBuf::new(SCENE_PATH);

	if CliArgs::parse_env().params.contains_key("clear") {
		fs_ext::remove(asset_path.into_abs()).ok();
	}

	if let Ok(scene) = fs_ext::read(asset_path.into_abs()) {
		// scene exists, clear
		commands.queue(move |world: &mut World| -> Result {
			SceneLoader::new(world).load_json(&scene)?;
			let root = world
				.query_filtered_once::<Entity, With<Root>>()
				.into_iter()
				.next()
				.expect("root not found in loaded scene");
			world
				.commands()
				.entity(root)
				.call::<(), Outcome>((), default());
			Ok(())
		});
	} else {
		commands
			.spawn((Root, Repeat::new(), children![(
				Thread::default(),
				Sequence::new().allow_no_tool(),
				children![
					(Actor::system(), children![Post::spawn(
						"Get to know the user as well as possible, who are they?
						your responses should be brief"
					)]),
					(
						Actor::new("Agent", ActorKind::Agent),
						// OllamaProvider::qwen()
						OpenAiProvider::gpt_5_mini().unwrap()
					),
					// save after user post
					SaveScene,
					(Actor::new("User", ActorKind::User), StdinPost),
					// save again after agent post
					SaveScene
				]
			),]))
			.call::<(), Outcome>((), default());
	}
}


#[derive(Component, Reflect)]
#[reflect(Component)]
struct Root;


/// On each loop, saves the scene to the asset path
#[tool]
#[derive(Component, Reflect)]
#[reflect(Component)]
fn SaveScene(_: SystemToolIn, world: &mut World) -> Result<Outcome> {
	let root = world
		.query_filtered_once::<Entity, With<Root>>()
		.into_iter()
		.next()
		.ok_or_else(|| bevyhow!("Root entity not found for SaveScene"))?;
	let json = SceneSaver::new(world).with_entity_tree(root).save_json()?;
	fs_ext::write(WsPathBuf::new(SCENE_PATH).into_abs(), json)?;
	Ok(PASS)
}
