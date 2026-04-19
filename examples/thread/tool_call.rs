//! Demonstrates tool calling
use beet::prelude::*;

#[beet::main]
async fn main() {
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
	let schema = reflect_ext::json_schema::<ChoiceInput>();
	println!("Tool Call Schema:\n{:#?}", schema);

	commands
		.spawn((RepeatTimes::<()>::new(2), children![(
			Thread::default(),
			Sequence::new(),
			ExcludeErrors(ChildError::NO_ACTION),
			children![
				(Actor::system(), children![Post::spawn(
					r#"
This is a tool call test. You will get two responses.

For the first, respond in the first person:
> "I open the door.." - good, you are rolepaying
> "The door opens, what do you want to do next" - bad, remember you are a participant, not dungeon master

For the second, if you receive a tool output, produce a final text response.
If you instead receive an error break character,
describe the error to the user, and offer a suggestion to fix it.
This will be your final response so do not attempt to continue the conversation.

## Scenario

You enter the cave, and from the ceiling drops a glowing red beet..
"#
				)]),
				(
					Actor::new("Fearless Warrior", ActorKind::Agent),
					// OllamaProvider::default_12gb(),
					OpenAiProvider::gpt_5_mini().unwrap(),
					children![AgentChoiceAction]
				),
			]
		),]))
		.call::<(), Outcome>((), OutHandler::exit());
}

/// Make a choice for what to do, following the schema
#[action(pure, route = "make-choice")]
#[derive(Component, Reflect)]
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
