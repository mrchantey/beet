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
		.add_observer(listen_for_changes)
		.run();
}

fn create_scene(mut commands: Commands, mut query: ContextQuery) -> Result {
	// 1. define actors
	let clanker_id = query.add_actor(Actor::clanker());
	let system_id = query.add_actor(Actor::system());
	let user_id = query.add_actor(Actor::user());
	// 2. define relations
	commands.spawn((system_id, children![
		(clanker_id, ModelAction::new(OllamaProvider::default())),
		user_id
	]));

	// 3. define items
	query.add_items([Item::new(
		system_id,
		Content::message("you are robot, make beep boop noises"),
		ItemScope::single_actor(clanker_id),
	)])?;
	Ok(())
}

fn run_clanker(mut commands: Commands, query: ContextQuery) {
	let clanker = query
		.actors()
		.find(|actor| actor.kind() == ActorKind::Agent)
		.unwrap();

	let clanker_entity = *query.actor_entities(clanker.id()).first().unwrap();

	// println!("Running clanker with input: {input:#?}");
	commands
		.entity(clanker_entity)
		.call::<(), ()>((), default());
}

fn listen_for_changes(
	ev: On<ItemsAdded>,
	mut commands: Commands,
	context_query: ContextQuery,
) -> Result {
	for item in &ev.items {
		let item = context_query.item(*item)?;
		let actor = context_query.actor(item.owner())?;
		println!("{} > {:?}", actor.name(), item.content());
	}
	// commands.write_message(AppExit::Success);
	Ok(())
}
