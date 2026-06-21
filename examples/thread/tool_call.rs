//! Tool calling: an inline `#[action]` referenced by tag in a `.bsx` scene, run
//! to completion and rendered through the agnostic charcell UI.
use beet::prelude::*;

/// The author scene: a dungeon turn whose agent is equipped with `<AgentChoiceAction/>`.
const SCENE: &str = include_str!("tool_call.bsx");

#[beet::main]
async fn main() {
	env_ext::load_dotenv();
	App::new()
		.add_plugins((
			MinimalPlugins,
			ThreadPlugin::default(),
			ThreadUiPlugin,
			CharcellTuiPlugin,
		))
		// register the inline tool so `<AgentChoiceAction/>` resolves from markup
		.register_type::<AgentChoiceAction>()
		.add_systems(Startup, setup)
		.run();
}

fn setup(async_commands: AsyncCommands) {
	async_commands.run(async move |world: AsyncWorld| -> Result {
		// reduce the scene and mount the transcript, all before the turn runs
		let root = world
			.with(|world: &mut World| -> Result<Entity> {
				let root =
					BsxTemplate::parse_entry(world, SCENE)?.spawn(world)?;
				reduce_threads_now(world);
				let thread = world
					.query_filtered::<Entity, With<Thread>>()
					.iter(world)
					.next()
					.ok_or_else(|| bevyhow!("no Thread in scene"))?;
				world.spawn(thread_tui(thread));
				Ok(root)
			})
			.await?;
		world.entity(root).call::<(), Outcome>(()).await?;
		world.write_message(AppExit::Success).await;
		Ok(())
	});
}

/// Make a choice for what to do, following the schema
#[action(pure, route = "make-choice")]
#[derive(Component, Reflect)]
#[reflect(Component)]
fn AgentChoiceAction(cx: ActionContext<ChoiceInput>) -> String {
	match cx.choice {
		Choice::Attack => {
			"the attack was successful, you must feel very smug.."
		}
		Choice::Defend => "you exhibited cowardice, the shame..",
		Choice::GreetWarmly => {
			"its almost as if the glowing beet winked in response.."
		}
	}
	.to_string()
}

#[derive(Reflect, serde::Deserialize, serde::Serialize)]
struct ChoiceInput {
	/// The choice you can make, follow the schema and pick one.
	choice: Choice,
	/// A line of dialog to say as you make your choice
	catchphrase: String,
}
#[derive(Reflect, serde::Deserialize, serde::Serialize)]
enum Choice {
	Attack,
	/// Do Thing
	Defend,
	GreetWarmly,
}
