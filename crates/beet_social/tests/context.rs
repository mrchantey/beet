#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
use beet_core::prelude::*;
use beet_social::prelude::*;
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
		.init_plugin::<SocialPlugin>()
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands) {
	commands
		.spawn((
			Thread::default(),
			Sequence::new().allow_no_tool(),
			children![
				(User::system(), children![Post::spawn(
					"you are robot, make beep boop noises"
				)]),
				(User::agent(), post_tool(OllamaProvider::qwen_3_8b())),
				(system_tool(assert_and_exit))
			],
		))
		.call::<(), Outcome>((), default());
}


fn assert_and_exit(
	input: In<SystemToolIn>,
	mut commands: Commands,
	query: ThreadQuery,
) -> Result<Outcome> {
	let view = query.view(input.caller)?;
	view.posts
		.into_iter()
		.find(|post| {
			post.payload().kind() == PostKind::Text
				&& post.user.kind() == UserKind::Agent
		})
		.unwrap()
		.payload()
		.to_string()
		.to_lowercase()
		.xpect_contains("beep");

	commands.write_message(AppExit::Success);
	Ok(Pass(()))
}
