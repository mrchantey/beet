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
		.spawn((Repeat::new(), children![(
			Thread::default(),
			Sequence::new(),
			ExcludeErrors(ChildError::NO_ACTION),
			children![
				(Actor::system(), children![Post::spawn(
					"this is a roleplay, keep responses under 50 words."
				)]),
				(
					Actor::new("Beet lover", ActorKind::Agent),
					OpenAiProvider::gpt_5_mini()
						.unwrap()
						.with_instructions("You love beets")
				),
				(
					Actor::new("Beet disliker", ActorKind::Agent),
					OpenAiProvider::gpt_5_mini().unwrap().with_instructions(
						"You think beets are bad but dont want to hurt feelings"
					)
				),
				// (Actor::new("Some random blow-in", ActorKind::User), StdinPost),
			]
		),]))
		.call::<(), Outcome>((), default());
}
