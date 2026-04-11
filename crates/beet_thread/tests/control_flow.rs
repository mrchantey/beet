#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
use beet_core::prelude::*;
use beet_thread::prelude::*;
use beet_tool::prelude::*;

#[ignore = "requires Ollama running locally"]
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
		.init_plugin::<ThreadPlugin>()
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands) {
	commands
		.spawn((
			Thread::default(),
			Sequence::new().allow_no_tool(),
			children![
				(Actor::system(), children![Post::spawn(
					"you are robot, make beep boop noises"
				)]),
				(Actor::agent(), OllamaProvider::qwen()),
				(Tool::<(), Outcome>::new_system(assert_and_exit))
			],
		))
		.call::<(), Outcome>((), default());
}


fn assert_and_exit(
	input: In<ToolContext>,
	mut commands: Commands,
	query: ThreadQuery,
) -> Result<Outcome> {
	let view = query.thread(input.id())?;
	view.posts
		.iter()
		.find(|post| {
			post.intent().is_display() && post.actor.kind() == ActorKind::Agent
		})
		.unwrap()
		.to_string()
		.to_lowercase()
		.xpect_contains("beep");

	commands.write_message(AppExit::Success);
	Ok(Pass(()))
}
