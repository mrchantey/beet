// //! # Clanker Chat
fn main() {}

// use beet::prelude::*;

// fn main() {
// 	App::new()
// 		.add_plugins((MinimalPlugins, LogPlugin {
// 			// level: Level::TRACE,
// 			..default()
// 		}))
// 		.init_resource::<StdoutCursor>()
// 		.init_plugin::<ClankerPlugin>()
// 		.add_systems(Startup, create_scene)
// 		.add_observer(log_name)
// 		.add_observer(log_delta)
// 		.run();
// }



// fn create_scene(mut commands: Commands, mut query: ActionQuery) -> Result {
// 	// 1. define actors
// 	let clanker_id = query.actors_mut().insert(Actor::agent());
// 	let system_id = query.actors_mut().insert(Actor::system());
// 	let user_id = query.actors_mut().insert(Actor::user());

// 	let thread_id = query.threads_mut().insert(Thread::default());

// 	// 2. define relations
// 	commands
// 		.spawn((Repeat::new(), children![(
// 			system_id,
// 			Sequence::new(),
// 			children![
// 				(
// 					clanker_id,
// 					thread_id,
// 					ModelAction::new(OllamaProvider::default()).streaming()
// 				),
// 				(user_id, thread_id, stdin.into_tool())
// 			]
// 		)]))
// 		.call::<(), Outcome>((), default());

// 	// 3. define items
// 	query.add_actions(Action::new(
// 		system_id,
// 		thread_id,
// 		ActionStatus::Completed,
// 		"you are robot, make beep boop noises",
// 	))?;
// 	Ok(())
// }

// #[tool]
// fn stdin(
// 	input: SystemToolIn,
// 	mut query: ActionQuery,
// 	actors: Query<(&ActorId, &ThreadId)>,
// ) -> Result<Outcome> {
// 	let (actor, thread) = actors.get(input.caller)?;
// 	let mut input = String::new();
// 	let heading = paint_ext::cyan_bold(format!("\n\nUser > "));
// 	print!("{heading}");
// 	std::io::Write::flush(&mut std::io::stdout())?;
// 	std::io::stdin().read_line(&mut input)?;
// 	query.add_actions(Action::new(
// 		*actor,
// 		*thread,
// 		ActionStatus::Completed,
// 		input,
// 	))?;
// 	Ok(Pass(()))
// }

// // cursor to track which part of action deltas have already been printed
// #[derive(Default, Deref, DerefMut, Resource)]
// struct StdoutCursor(HashMap<ActionId, u32>);

// fn log_name(ev: On<ActionCreated>, context_query: ActionQuery) -> Result {
// 	let actor = context_query.actors().get(ev.actor)?;
// 	if actor.kind() != ActorKind::Agent {
// 		return Ok(());
// 	}
// 	let action = context_query.actions().get(ev.action)?;
// 	let action_kind = action.payload().kind();
// 	if !action_kind.is_display() {
// 		return Ok(());
// 	}
// 	let actor = context_query.actors().get(action.author())?;

// 	use ActionKind::*;
// 	let suffix = match action_kind {
// 		Refusal => "refusal >",
// 		ReasoningSummary | ReasoningContent | ReasoningEncryptedContent => {
// 			"thinking.. "
// 		}
// 		Media | Url => "media ",
// 		_ => ">",
// 	};

// 	let heading =
// 		paint_ext::cyan_bold(format!("\n{} {}\n", actor.name(), suffix));
// 	println!("{heading}");
// 	Ok(())
// }

// fn log_delta(
// 	ev: On<ActionUpdated>,
// 	context_query: ActionQuery,
// 	mut cursor: ResMut<StdoutCursor>,
// ) -> Result {
// 	let actor = context_query.actors().get(ev.actor)?;
// 	if actor.kind() != ActorKind::Agent {
// 		return Ok(());
// 	}
// 	let action = context_query.actions().get(ev.action)?;
// 	if !action.payload().kind().is_display() {
// 		return Ok(());
// 	}
// 	let payload = action.payload().to_string();
// 	let cursor_item = cursor.entry(ev.action).or_insert(0);

// 	let new_content = &payload[*cursor_item as usize..];
// 	use ActionKind::*;
// 	let colored = match action.payload().kind() {
// 		Refusal => paint_ext::red(new_content),
// 		ReasoningSummary | ReasoningContent | ReasoningEncryptedContent => {
// 			paint_ext::dimmed(new_content)
// 		}
// 		_ => new_content.to_string(),
// 	};

// 	print!("{}", colored);
// 	*cursor_item = payload.len() as u32;

// 	Ok(())
// }
