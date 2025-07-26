use beet_core::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
pub mod realtime;


pub struct OpenAiKey;

impl OpenAiKey {
	/// Load the `OPENAI_API_KEY` from the environment variables.
	pub fn get() -> Result<String> {
		std::env::var("OPENAI_API_KEY")?.xok()
	}
}
