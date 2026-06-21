//! Tool calling: an inline `#[action]` referenced by tag in a `.bsx` scene, run
//! to completion and rendered through the agnostic charcell UI.
//!
//! The scene is the whole program: a dungeon turn whose agent is equipped with
//! `<AgentChoiceAction/>`, looped twice by `RepeatTimes` and kicked by
//! `{RunThread}`. `main` just loads it and registers the inline tool type so the
//! `<AgentChoiceAction/>` tag resolves from markup.
use beet::prelude::*;

const SCENE: &str = include_str!("tool_call.bsx");

fn main() {
	env_ext::load_dotenv();
	App::new()
		.add_plugins((
			MinimalPlugins,
			ThreadPlugin::default(),
			ThreadUiPlugin,
			CharcellTuiPlugin,
			ThreadScenePlugin::new(SCENE),
		))
		// register the inline tool so `<AgentChoiceAction/>` resolves from markup
		.register_type::<AgentChoiceAction>()
		.run();
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
