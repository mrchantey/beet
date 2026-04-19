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
		.spawn((bucket, children![(
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
					]
				),
			]
		)]))
		.call::<(), Outcome>((), OutHandler::exit());
}

const PROMPT: &str = r#"
You are a coding agent with access to a file bucket.

Create a beautiful single-page HTML file called `index.html` that showcases
a "Beet Framework" landing page.

Requirements:
- Use inline CSS (no external dependencies)
- Gradient background from deep purple to dark blue
- Hero section with the title "Beet Framework" and subtitle "Build anything, modify everything"
- Three feature cards with emoji icons for: Composable Actions, Cross-Platform, Live Editing
- A footer with "Powered by Beet"
- Responsive design using flexbox

Use the write-blob tool to create the file, passing the HTML as UTF-8 bytes.
Then use read-blob to verify the file was written correctly.
"#;
