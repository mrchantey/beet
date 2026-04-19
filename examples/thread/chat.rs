use beet::prelude::*;

fn main() {
	env_ext::load_dotenv();
	App::new()
		.add_plugins((
			MinimalPlugins,
			ThreadPlugin::default(),
			// logs all agent messages to stdout
			ThreadStdoutPlugin::default(),
		))
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands) {
	commands
		.spawn((Repeat, children![(
			Thread::default(),
			ExcludeErrors(ChildError::NO_ACTION),
			Sequence::new(),
			children![
				(Actor::system(), children![Post::spawn(
					"you are robot, make beep boop noises"
				)]),
				(
					Actor::new("BeepBot", ActorKind::Agent),
					OpenAiProvider::gpt_5_mini().unwrap()
				),
				(Actor::new("Billy", ActorKind::User), StdinPost),
			]
		),]))
		.call::<(), Outcome>((), default());
}
