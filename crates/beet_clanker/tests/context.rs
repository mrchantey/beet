#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]

use beet_clanker::prelude::*;
use beet_core::prelude::*;


#[beet_core::test(timeout_ms = 15_000)]
fn main() {
	App::new()
		.init_plugin::<ClankerPlugin>()
		.add_systems(Startup, create_scene)
		.add_systems(PostStartup, run_clanker)
		.run();
}

fn create_scene(mut commands: Commands, mut query: ContextQuery) -> Result {
	let system = Actor::system();
	let clanker = Actor::clanker();
	let user = Actor::user();
	let item = Item::new(
		system.id(),
		Content::text("you are robot, make beep boop noises"),
		ItemScope::single_actor(clanker.id()),
	);

	let clanker = query.spawn_actor(clanker).id();
	let user = query.spawn_actor(user).id();
	// query.spawn_actor(system).add_child(clanker).add_child(user);
	query.add_item(item)?;
	Ok(())
}

fn run_clanker(
	mut commands: Commands,
	query: ContextQuery,
	actors: Query<&Actor>,
) {
	let clanker = actors
		.iter()
		.find(|actor| actor.kind() == ActorKind::Agent)
		.unwrap();

	let input = ContextBuilder::default()
		.build(&query, clanker.id())
		.unwrap();


	commands.queue_async(async |world| {});
}
