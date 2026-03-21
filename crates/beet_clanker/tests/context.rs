#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]

use beet_clanker::prelude::*;
use beet_core::prelude::*;
use beet_tool::prelude::*;


#[beet_core::test(timeout_ms = 15_000)]
fn main() {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin {
			level: Level::TRACE,
			filter: format!("{}=trace,ureq=off,ureq_proto=off", module_path!()),
			..default()
		}))
		.init_plugin::<ClankerPlugin>()
		.add_systems(Startup, setup)
		// .add_observer(|_ev: On<ActionCreated>| {
		// 	println!("action created");
		// })
		// .add_observer(|_ev: On<ActionUpdated>| {
		// 	println!("action updated");
		// })
		.run();
}

fn setup(mut commands: Commands, mut query: ContextQuery) -> Result {
	// 1. define actors
	let thread = Thread::default();
	let thread_id = thread.id();

	let clanker = Actor::agent();
	let clanker_id = clanker.id();
	let system = Actor::system();
	let system_id = system.id();
	let user = Actor::user();
	let user_id = user.id();

	let store = ActionStore::default();

	async_ext::block_on(async {
		store.insert_thread(thread.clone()).await.unwrap();
		store.insert_actor(clanker.clone()).await.unwrap();
		store.insert_actor(system.clone()).await.unwrap();
		store.insert_actor(user.clone()).await.unwrap();

		store
			.insert_actions(&vec![&Action::new(
				system_id,
				thread_id,
				ActionStatus::Completed,
				"you are robot, make beep boop noises",
			)])
			.await
			.unwrap()
	});

	// register actors and thread in the ContextMap resource
	query.threads_mut().insert(thread);
	query.actors_mut().insert(clanker);
	query.actors_mut().insert(system);
	query.actors_mut().insert(user);

	// 2. define relations
	commands
		.spawn((system_id, Sequence::new(), children![
			(
				store,
				clanker_id,
				thread_id,
				action_tool(OllamaProvider::qwen_3_8b()),
				// ModelAction::new(OllamaProvider::default()).streaming()
			),
			(user_id, thread_id, exit_on_user_turn.into_tool())
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
	commands.write_message(AppExit::Success);
	Pass(())
}
