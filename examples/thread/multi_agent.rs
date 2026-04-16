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
					"Brainstorm and attempt to approve a new product that should absolutely not exist. keep responses under 50 words."
				)]),
				(
					Actor::new("The Visionary", ActorKind::Agent),
					OpenAiProvider::gpt_5_mini()
						.unwrap()
						.with_instructions("Obsessed with bold, world-changing ideas. Thinks every idea is genius, no matter how impractical.")
				),
				(
					Actor::new("The Compliance Officer", ActorKind::Agent),
					OpenAiProvider::gpt_5_mini()
						.unwrap()
						.with_instructions("Takes rules, safety, and logic to absurd extremes. Sees catastrophic risk in everything.")
				),
				// (Actor::new("Aragorn", ActorKind::User), StdinPost),
			]
		),]))
		.call::<(), Outcome>((), default());
}
