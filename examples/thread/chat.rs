use beet::prelude::*;

fn main() {
	env_ext::load_dotenv();
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin {
			// level: Level::DEBUG,
			level: Level::TRACE,
			filter: format!("bevy_time=off,ureq=off,ureq_proto=off"),
			..default()
		}))
		.init_plugin::<ActorPlugin>()
		.add_systems(Startup, setup)
		.add_systems(PostUpdate, (on_create, on_change).chain())
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
					// OllamaProvider::qwen_3_8b()
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
	mut query: SocialQuery,
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

// cursor to track which part of post deltas have already been printed
#[derive(Default, Deref, DerefMut, Component)]
struct StdoutCursor(u32);

fn on_create(
	mut commands: Commands,
	query: Populated<(Entity, &Post), Added<Post>>,
	thread_query: SocialQuery,
) -> Result {
	for (entity, post) in query.iter() {
		commands.entity(entity).insert(StdoutCursor::default());
		let actor = thread_query.actor_from_post_entity(entity)?;

		if actor.kind() != ActorKind::Agent {
			continue;
		}

		// hide reasoning in release builds
		#[cfg(not(debug_assertions))]
		if !post.intent().is_display() {
			continue;
		}

		let agent_post = post.as_agent_post();
		let suffix = if agent_post.is_refusal() {
			"refusal >"
		} else if agent_post.is_reasoning_summary()
			|| agent_post.is_reasoning_content()
		{
			"thinking.. "
		} else if agent_post.is_url() || agent_post.is_bytes() {
			"media "
		} else {
			">"
		};

		let heading =
			paint_ext::cyan_bold(format!("\n{} {}\n", actor.name(), suffix));
		println!("{heading}");
	}

	Ok(())
}

fn on_change(
	mut query: Populated<(Entity, &Post, &mut StdoutCursor), Changed<Post>>,
	thread_query: SocialQuery,
) -> Result {
	for (entity, post, mut cursor) in query.iter_mut() {
		let actor = thread_query.actor_from_post_entity(entity)?;
		if actor.kind() != ActorKind::Agent {
			continue;
		}

		// hide reasoning in release builds
		#[cfg(not(debug_assertions))]
		if !post.intent().is_display() {
			continue;
		}
		let body = post.to_string();

		let new_content = &body[**cursor as usize..];
		let agent_post = post.as_agent_post();
		let colored = if agent_post.is_refusal() {
			paint_ext::red(new_content)
		} else if agent_post.is_reasoning_summary()
			|| agent_post.is_reasoning_content()
		{
			paint_ext::dimmed(new_content)
		} else {
			new_content.to_string()
		};

		print!("{}", colored);
		**cursor = body.len() as u32;
	}

	Ok(())
}
