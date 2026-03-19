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
	let clanker_id = query.actors_mut().insert(Actor::clanker());
	let system_id = query.actors_mut().insert(Actor::system());
	let user_id = query.actors_mut().insert(Actor::user());

	// clanker thread is the thread sent to the model
	let clanker_thread = query.threads_mut().insert(
		Thread::default().with_actors([system_id, clanker_id, user_id]),
	);

	// user thread is the thread printed to stdout
	let user_thread = query
		.threads_mut()
		.insert(Thread::display_only().with_actors([clanker_id, user_id]));

	// 2. define relations
	commands
		.spawn((system_id, Sequence::default(), children![
			(
				clanker_id,
				clanker_thread,
				ModelAction::new(OllamaProvider::default()).streaming()
			),
			(
				user_id,
				user_thread,
				StdoutCursor::default(),
				OnSpawn::observe(log_clanker_name),
				OnSpawn::observe(log_clanker_delta),
				exit_on_user_turn.into_tool()
			)
		]))
		.call::<(), Outcome>((), default());

	// 3. define items
	query.add_items(Item::new(
		system_id,
		ItemStatus::Completed,
		"you are robot, make beep boop noises",
	))?;
	Ok(())
}


fn log_clanker_name(
	ev: On<EntityItemCreated>,
	context_query: ContextQuery,
) -> Result {
	let item = context_query.items().get(ev.item)?;
	let actor = context_query.actors().get(item.owner())?;
	println!("<< {} >> ", actor.name());
	Ok(())
}


#[allow(unused)]
#[derive(Default, Component)]
struct StdoutCursor(HashMap<ItemId, u32>);


fn log_clanker_delta(
	ev: On<EntityItemUpdated>,
	context_query: ContextQuery,
	mut query: Query<&mut StdoutCursor>,
) -> Result {
	let mut cursor = query.get_mut(ev.entity)?;
	let item = context_query.items().get(ev.item)?;
	let content = item.content().to_string();
	let cursor_item = cursor.0.entry(ev.item).or_insert(0);

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
	let item = context_query
		.items()
		.values()
		.find(|item| {
			item.content().kind() == ItemKind::Text
				&& context_query.actors().get(item.owner()).unwrap().kind()
					== ActorKind::Agent
		})
		.unwrap();
	item.content()
		.to_string()
		.to_lowercase()
		.xpect_contains("beep");
	println!("");
	commands.write_message(AppExit::Success);
	Pass(())
}
