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
			Sequence::new().allow_no_tool(),
			children![
				(Actor::system(), children![Post::spawn(
					"you are robot, make beep boop noises"
				)]),
				(
					Actor::new("BeepBot", ActorKind::Agent),
					OpenAiProvider::gpt_5_mini().unwrap()
				),
				(
					Actor::new("Billy", ActorKind::Human),
					stdin_post_tool.into_tool()
				),
			]
		),]))
		.call::<(), Outcome>((), default());
}

#[tool]
fn stdin_post_tool(
	cx: SystemToolIn,
	mut query: ThreadQuery,
	actors: Query<&Actor>,
) -> Result<Outcome> {
	let actor = actors.get(cx.caller)?;
	let heading = paint_ext::cyan_bold(format!("\n\n{} > ", actor.name()));
	print!("{heading}");
	std::io::Write::flush(&mut std::io::stdout())?;
	let mut input = String::new();
	std::io::stdin().read_line(&mut input)?;
	query.spawn_post(cx.caller, PostStatus::Completed, input)?;
	Ok(Pass(()))
}
