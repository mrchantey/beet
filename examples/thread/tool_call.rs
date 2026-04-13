//! Demonstrates tool calling
use beet::prelude::*;

#[beet::main]
async fn main() {
	let schema = reflect_ext::json_schema::<MakeChoice>();
	println!("schema: {:#?}", schema);
	env_ext::load_dotenv();
	App::new()
		.add_plugins((
			MinimalPlugins,
			ThreadPlugin::default(),
			ThreadStdoutPlugin::default(),
		))
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands) {
	commands
		.spawn((RepeatTimes::<()>::new(2), children![(
			Thread::default(),
			Sequence::new().allow_no_tool(),
			children![
				(Actor::system(), children![Post::spawn(
					r#"
Respond in the first person:
> "I open the door.." - good, you are rolepaying
> "The door opens, what do you want to do next" - bad, remember you are a participant, not dungeon master

If you do not receive the tool call output for your second response,
break character, describe the error to the user, and offer a suggestion to fix it.

## Scenario

You enter the cave, and from the ceiling drops a glowing red beet..
"#
				)]),
				(
					Actor::new("Fearless Warrior", ActorKind::Agent),
					OpenAiProvider::gpt_5_mini().unwrap(),
					children![agent_choice_tool()]
				),
			]
		),]))
		.call::<(), Outcome>((), OutHandler::exit());
}

fn agent_choice_tool() -> impl Bundle {
	function_tool(
		"make-choice",
		"make your choice",
		Tool::<MakeChoice, String>::new_pure(|cx: ToolContext<MakeChoice>| {
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
			.xok()
		}),
	)
}


#[derive(Reflect, serde::Deserialize, serde::Serialize)]
struct MakeChoice {
	/// The choice you can make
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
