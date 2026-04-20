#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;
use beet_thread::prelude::*;

const PROMPT: &str = r#"
You are a testing agent. Perform these steps in order using the provided tools:

1. Use write-blob to create "test.txt" with content "hello beet"
2. Use read-blob to verify it contains "hello beet"
3. Use edit-text to change "hello beet" to "hello world"
4. Use read-blob to confirm the edit worked
5. Use write-blob to create "result.txt" with the content of what you read in step 4

After completing all steps, call execution-outcome with passed=true.
If any step fails, call execution-outcome with passed=false and describe in reason.

Do NOT delete any files.
"#;

#[ignore = "requires OpenAI API key"]
#[beet_core::test(timeout_ms = 60_000)]
fn main() {
	env_ext::load_dotenv();
	let dir = TempDir::new_ws().unwrap();
	let bucket_path = dir.path().clone();
	let verify_path = bucket_path.join("result.txt");

	App::new()
		.add_plugins((MinimalPlugins, ThreadPlugin::default()))
		.add_systems(Startup, move |mut commands: Commands| {
			let bucket = FsBucket::new(bucket_path.clone());

			commands
				.spawn((
					bucket,
					RepeatWhileFunctionCallOutput,
					children![(
						Thread::default(),
						Sequence::new(),
						ExcludeErrors(ChildError::NO_ACTION),
						children![
							(
								Actor::system(),
								children![Post::spawn(PROMPT)]
							),
							(
								Actor::new("Tester", ActorKind::Agent),
								OpenAiProvider::gpt_5_mini().unwrap(),
								children![
									exchange_route(
										"list-blobs",
										ListBlobs
									),
									exchange_route(
										"read-blob",
										ReadBlob
									),
									exchange_route(
										"write-blob",
										WriteBlob
									),
									exchange_route(
										"edit-text",
										EditText
									),
									exchange_route(
										"remove-blob",
										RemoveBlob
									),
									exchange_route(
										"execution-outcome",
										ExecutionOutcome
									),
								]
							),
						]
					)],
				))
				.call::<(), Outcome>((), OutHandler::exit());
		})
		.run();

	// verify the agent created result.txt with the edited content
	let result = fs_ext::read_to_string(&verify_path).unwrap();
	result.xpect_contains("hello world");
}
