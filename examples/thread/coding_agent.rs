//! Mini coding agent that creates HTML files in a local bucket.
//!
//! Run with:
//! ```sh
//! cargo run --example coding_agent --features thread,fs
//! ```
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
	let bucket = FsBucket::new(
		AbsPathBuf::new_workspace_rel(".beet/coding_agent").unwrap(),
	);

	commands
		.spawn((
			bucket,
			RepeatWhileFunctionCallOutput,
			children![(
				Thread::default(),
				Sequence::new(),
				ExcludeErrors(ChildError::NO_ACTION),
				children![
					(Actor::system(), children![Post::spawn(PROMPT)]),
					(
						Actor::new("Coder", ActorKind::Agent),
						OpenAiProvider::gpt_5_mini().unwrap(),
						children![
							route("list-blobs", ListBlobs),
							route("read-blob", ReadBlob),
							route("write-blob", WriteBlob),
							route("edit-text", EditText),
							route("remove-blob", RemoveBlob),
						]
					),
				]
			)],
		))
		.call::<(), Outcome>((), OutHandler::exit());
}

const PROMPT: &str = r#"
You are a coding agent with access to a file bucket.
Your available tools are: list-blobs, read-blob, write-blob, edit-text, remove-blob.

We are testing that your tool calls work correctly.
Please perform the following steps in order:

1. Use list-blobs (path: "") to see what's in the bucket
2. Use write-blob to create a file called "hello.txt" with the content "Hello, Beet!"
   (pass the text as UTF-8 bytes)
3. Use read-blob to verify the file was written correctly
4. Use edit-text to change "Hello, Beet!" to "Hello, World!"
5. Use read-blob again to confirm the edit
6. Use remove-blob to delete "hello.txt"
7. Use list-blobs to confirm it was deleted
8. Finally, write a brief summary of what happened and whether all tools worked correctly
"#;
