
use beet_utils::prelude::*;
use bevy::prelude::*;
pub mod realtime;


pub struct OpenAiKey;

impl OpenAiKey {
	/// Load the `OPENAI_API_KEY` from the environment variables.
	pub fn get() -> Result<String> { env_ext::var("OPENAI_API_KEY")?.xok() }
}
