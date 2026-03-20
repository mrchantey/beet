#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]

use beet_clanker::prelude::*;
use beet_core::prelude::*;
use beet_tool::prelude::*;


#[beet_core::test(timeout_ms = 15_000)]
fn main() {
	App::new()
		.add_plugins(MinimalPlugins)
		.init_plugin::<ClankerPlugin>()
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands, mut query: ContextQuery) -> Result {
	// 1. define actors
	let clanker_id = query.actors_mut().insert(Actor::agent());
	let system_id = query.actors_mut().insert(Actor::system());
	let user_id = query.actors_mut().insert(Actor::user());

	// clanker thread is the thread sent to the model
	let thread_id = query.threads_mut().insert(Thread::default());

	// 2. define relations
	commands
		.spawn((system_id, Sequence::new(), children![
			(
				clanker_id,
				thread_id,
				ModelAction::new(OllamaProvider::default()).streaming()
			),
			(
				user_id,
				thread_id,
				StdoutCursor::default(),
				OnSpawn::observe(log_clanker_name),
				OnSpawn::observe(log_clanker_delta),
				exit_on_user_turn.into_tool()
			)
		]))
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


fn log_clanker_name(
	ev: On<EntityActionCreated>,
	context_query: ContextQuery,
) -> Result {
	let action = context_query.actions().get(ev.action)?;
	let actor = context_query.actors().get(action.author())?;
	println!("<< {} >> ", actor.name());
	Ok(())
}


#[allow(unused)]
#[derive(Default, Component)]
struct StdoutCursor(HashMap<ActionId, u32>);


fn log_clanker_delta(
	ev: On<EntityActionUpdated>,
	context_query: ContextQuery,
	mut query: Query<&mut StdoutCursor>,
) -> Result {
	let mut cursor = query.get_mut(ev.entity)?;
	let action = context_query.actions().get(ev.action)?;
	let content = action.payload().to_string();
	let cursor_item = cursor.0.entry(ev.action).or_insert(0);

	let new_content = &content[*cursor_item as usize..];
	print!("{}", new_content);
	*cursor_item = content.len() as u32;

	Ok(())
}

#[tool]
fn exit_on_user_turn(
	_val: In<()>,
	mut commands: Commands,
	context_query: ContextQuery,
) -> Outcome {
	let text_action = context_query
		.actions()
		.values()
		.find(|action| {
			action.payload().kind() == ActionKind::Text
				&& context_query.actors().get(action.author()).unwrap().kind()
					== ActorKind::Agent
		})
		.unwrap();
	text_action
		.payload()
		.to_string()
		.to_lowercase()
		.xpect_contains("beep");
	println!("");
	commands.write_message(AppExit::Success);
	Pass(())
}
