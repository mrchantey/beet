//! # Clanker Chat
//!
//! - note that the ollama models like qwen3 will occasionally think only, without text output..
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin {
			// level: Level::TRACE,
			..default()
		}))
		.init_plugin::<ClankerPlugin>()
		.add_systems(Startup, create_scene)
		// .add_systems(PostStartup, run_clanker)
		// .add_observer(exit_on_complete)
		.run();
}



fn create_scene(mut commands: Commands, mut query: ContextQuery) -> Result {
	// 1. define actors
	let clanker_id = query.actors_mut().insert(Actor::agent());
	let system_id = query.actors_mut().insert(Actor::system());
	let user_id = query.actors_mut().insert(Actor::user());

	let thread_id = query.threads_mut().insert(Thread::default());

	// 2. define relations
	commands
		.spawn((Repeat::new(), children![(
			system_id,
			Sequence::new(),
			children![
				(
					clanker_id,
					thread_id,
					ModelAction::new(OllamaProvider::default()).streaming()
				),
				(
					user_id,
					thread_id,
					stdin.into_tool(),
					StdoutCursor::default(),
					OnSpawn::observe(log_name),
					OnSpawn::observe(listen_for_changes)
				)
			]
		)]))
		.call::<(), Outcome>((), default());

	// 3. define items
	query.add_actions(Action::new(
		system_id,
		thread_id,
		ActionStatus::Completed,
		"you are robot, make beep boop noises",
	))?;
	Ok(())
}

#[tool]
fn stdin(
	entity: SystemToolIn,
	mut query: ContextQuery,
	actors: Query<(&ActorId, &ThreadId)>,
) -> Result<Outcome> {
	let (actor, thread) = actors.get(entity.caller)?;
	let mut input = String::new();
	// let
	print!("\nUser > ");
	std::io::Write::flush(&mut std::io::stdout())?;
	std::io::stdin().read_line(&mut input)?;
	query.add_actions(Action::new(
		*actor,
		*thread,
		ActionStatus::Completed,
		input,
	))?;
	Ok(Pass(()))
}

#[allow(unused)]
#[derive(Default, Component)]
struct StdoutCursor(HashMap<ActionId, u32>);

fn log_name(
	ev: On<EntityActionCreated>,
	context_query: ContextQuery,
) -> Result {
	let action = context_query.actions().get(ev.action)?;
	let action_kind = action.payload().kind();
	if !action_kind.is_display() {
		return Ok(());
	}
	let actor = context_query.actors().get(action.author())?;

	use ActionKind::*;
	let suffix = match action_kind {
		Refusal => "refusal ",
		ReasoningSummary | ReasoningContent | ReasoningEncryptedContent => {
			"thinking.. "
		}
		Media | Url => "media ",
		_ => "",
	};

	let heading =
		paint_ext::cyan_bold(format!("{} {}>\n", actor.name(), suffix));
	println!("{heading}");
	Ok(())
}

fn listen_for_changes(
	ev: On<EntityActionUpdated>,
	context_query: ContextQuery,
	mut query: Query<&mut StdoutCursor>,
) -> Result {
	let mut cursor = query.get_mut(ev.entity)?;
	let action = context_query.actions().get(ev.action)?;
	if !action.payload().kind().is_display() {
		return Ok(());
	}
	let payload = action.payload().to_string();
	let cursor_item = cursor.0.entry(ev.action).or_insert(0);

	let new_content = &payload[*cursor_item as usize..];
	print!("{}", new_content);
	*cursor_item = payload.len() as u32;

	Ok(())
}
