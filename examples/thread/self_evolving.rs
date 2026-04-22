//! Demonstrates tool calling
use beet::prelude::*;

#[beet::main]
async fn main() {
	env_ext::load_dotenv();
	App::new()
		.add_plugins((
			MinimalPlugins,
			ThreadPlugin::default(),
			ThreadStdoutPlugin::default(),
		))
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands) {
	let bucket_path = WsPathBuf::new("target/examples/self_evolving");
	fs_ext::remove(&bucket_path).ok();
	let bucket = FsBucket::new(bucket_path);

	commands
		.spawn((RepeatTimes::new(10), bucket, children![(
			Thread::default(),
			Sequence::new(),
			ExcludeErrors(ChildError::NO_ACTION),
			children![
				(Actor::system(), children![Post::spawn(SYSTEM_PROMPT)]),
				// (Actor::user(), StdinPost, children![Post::spawn(USER_PROMPT)]),
				(
					Actor::agent(),
					// OllamaProvider::default_12gb(),
					OpenAiProvider::gpt_5_mini().unwrap(),
					children![
						exchange_route("list-blobs", ListBlobs),
						exchange_route("read-blob", ReadBlob),
						exchange_route("write-blob", WriteBlob),
						exchange_route("edit-text", EditText),
						exchange_route("remove-blob", RemoveBlob),
					]
				),
			]
		),]))
		.call::<(), Outcome>((), OutHandler::exit());
}

const SYSTEM_PROMPT: &'static str = r#"
TODO
"#;
