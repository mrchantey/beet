//! An example demonstrating high level image generation,
//! see the helper methods in `session_ext` for more detailed usage
use beet::prelude::*;

pub fn main() {
	// 1. construct the session
	let agent = GeminiAgent::from_env().with_model(GEMINI_2_5_FLASH_IMAGE);
	let message = session_ext::message(
		"image of game engine editor being used to control fun little robot characters in real life",
	);
	let session = session_ext::user_message_session(agent, message);

	// 2. run and collect output
	for (index, content) in session_ext::run_and_collect_file(session)
		.await
		.into_iter()
		.enumerate()
	{
		// 3. print and save output
		match content {
			ContentEnum::Text(text) => println!("Agent > {text}"),
			ContentEnum::File(file) => {
				let path = AbsPathBuf::new_workspace_rel(format!(
					"target/examples/image/file-{}-{index}.png",
					time_ext::now_millis()
				))
				.unwrap();
				println!("Agent > File: {}", path.display());
				let data = file.data.get().await.unwrap();
				fs_ext::write(&path, data).unwrap();
			}
		}
	}
}
