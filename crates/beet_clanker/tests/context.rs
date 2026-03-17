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
		.add_systems(Startup, create_scene)
		.add_systems(PostStartup, run_clanker)
		// .add_observer(listen_for_changes)
		.run();
}

fn create_scene(mut commands: Commands, mut query: ContextQuery) -> Result {
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
	commands.spawn((system_id, children![
		(
			clanker_id,
			clanker_thread,
			ModelAction::new(OllamaProvider::default())
		),
		(
			user_id,
			user_thread,
			StdoutCursor::default(),
			OnSpawn::observe(listen_for_changes)
		)
	]));

	// 3. define items
	query.add_items(Item::new(
		system_id,
		ItemStatus::Completed,
		"you are robot, make beep boop noises",
	))?;
	Ok(())
}

fn run_clanker(mut commands: Commands, query: ContextQuery) {
	// hack until beet_tool sequence
	let clanker = query
		.actors()
		.values()
		.find(|actor| actor.kind() == ActorKind::Agent)
		.unwrap();

	let clanker_entity = *query.actor_entities(clanker.id()).first().unwrap();

	// println!("Running clanker with input: {input:#?}");
	commands
		.entity(clanker_entity)
		.call::<(), ()>((), default());
}

#[allow(unused)]
#[derive(Default, Component)]
struct StdoutCursor(u32);

fn listen_for_changes(
	ev: On<EntityItemAdded>,
	// mut _commands: Commands,
	context_query: ContextQuery,
) -> Result {
	let item = context_query.items().get(ev.item)?;
	let actor = context_query.actors().get(item.owner())?;
	println!("{} > {}\n\n\n", actor.name(), item.content());
	// commands.write_message(AppExit::Success);
	Ok(())
}
