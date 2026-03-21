#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
use beet_clanker::prelude::*;
use beet_core::prelude::*;
use beet_tool::prelude::*;

#[beet_core::test(timeout_ms = 15_000)]
fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			// 	LogPlugin {
			// 	level: Level::TRACE,
			// 	filter: format!("{}=trace,ureq=off,ureq_proto=off", module_path!()),
			// 	..default()
			// }
		))
		.init_plugin::<ClankerPlugin>()
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands) -> Result {
	commands.queue_async(async |world| {
		// 1. define actors
		let store = ActionStore::default();
		let thread_id = store.insert_thread(Thread::default()).await?;
		let clanker_id = store.insert_actor(Actor::agent()).await?;
		let system_id = store.insert_actor(Actor::system()).await?;
		store
			.insert_actions(vec![Action::new(
				system_id,
				thread_id,
				ActionStatus::Completed,
				"you are robot, make beep boop noises",
			)])
			.await?;

		let entity = world
			.spawn_then((
				store.clone(),
				clanker_id,
				thread_id,
				action_tool(OllamaProvider::qwen_3_8b()),
			))
			.await;
		entity.call::<(), Outcome>(()).await?;

		let (action, _) = store
			.full_thread_actions(thread_id, None)
			.await?
			.into_iter()
			.find(|(action, actor)| {
				action.payload().kind() == ActionKind::Text
					&& actor.kind() == ActorKind::Agent
			})
			.unwrap();
		action
			.payload()
			.to_string()
			.to_lowercase()
			.xpect_contains("beep");

		world.write_message(AppExit::Success);
		Ok(())
	});
	Ok(())
}
