// //! # Clanker Chat

use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin {
			// level: Level::TRACE,
			..default()
		}))
		.init_plugin::<ClankerPlugin>()
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
				(Actor::system(), children![Action::spawn(
					"you are robot, make beep boop noises"
				)]),
				(
					Actor::agent(),
					action_tool(OllamaProvider::qwen_3_8b().with_instructions(
						r#"
To assist your understanding of the authorship of a post in multi-user environments.
you are provided input in xml format <actor name="foo" id="bar>{content}</action>.
Do not respond with this format.
"#
					))
				),
				(Actor::user(), stdin_action_tool.into_tool()),
			]
		),]))
		.call::<(), Outcome>((), default());
}

#[tool]
fn stdin_action_tool(
	cx: SystemToolIn,
	mut query: ThreadQuery,
) -> Result<Outcome> {
	let heading = paint_ext::cyan_bold(format!("\n\nUser > "));
	print!("{heading}");
	std::io::Write::flush(&mut std::io::stdout())?;
	let mut input = String::new();
	std::io::stdin().read_line(&mut input)?;
	query.spawn_action(cx.caller, ActionStatus::Completed, input)?;
	Ok(Pass(()))
}

// cursor to track which part of action deltas have already been printed
#[derive(Default, Deref, DerefMut, Component)]
struct StdoutCursor(u32);

fn on_create(
	mut commands: Commands,
	query: Populated<(Entity, &Action), Added<Action>>,
	thread_query: ThreadQuery,
) -> Result {
	for (entity, action) in query.iter() {
		commands.entity(entity).insert(StdoutCursor::default());
		let actor = thread_query.actor_from_action_entity(entity)?;

		if actor.kind() != ActorKind::Agent {
			continue;
		}
		let action_kind = action.payload().kind();
		if !action_kind.is_display() {
			continue;
		}

		use ActionKind::*;
		let suffix = match action_kind {
			Refusal => "refusal >",
			ReasoningSummary | ReasoningContent | ReasoningEncryptedContent => {
				"thinking.. "
			}
			Media | Url => "media ",
			_ => ">",
		};

		let heading =
			paint_ext::cyan_bold(format!("\n{} {}\n", actor.name(), suffix));
		println!("{heading}");
	}

	Ok(())
}

fn on_change(
	mut query: Populated<(Entity, &Action, &mut StdoutCursor), Changed<Action>>,
	thread_query: ThreadQuery,
) -> Result {
	for (entity, action, mut cursor) in query.iter_mut() {
		let actor = thread_query.actor_from_action_entity(entity)?;
		if actor.kind() != ActorKind::Agent {
			continue;
		}
		if !action.payload().kind().is_display() {
			continue;
		}
		let payload = action.payload().to_string();

		let new_content = &payload[**cursor as usize..];
		use ActionKind::*;
		let colored = match action.payload().kind() {
			Refusal => paint_ext::red(new_content),
			ReasoningSummary | ReasoningContent | ReasoningEncryptedContent => {
				paint_ext::dimmed(new_content)
			}
			_ => new_content.to_string(),
		};

		print!("{}", colored);
		**cursor = payload.len() as u32;
	}

	Ok(())
}
