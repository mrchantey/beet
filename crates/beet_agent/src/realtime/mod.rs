//! Communicate with a GPT-4o class model in real time using WebRTC or WebSockets. Supports text and audio inputs and ouputs, along with audio transcriptions.
//! ## References
//! [Guide](https://platform.openai.com/docs/guides/realtime)
//! [API Reference](https://platform.openai.com/docs/api-reference/realtime)
mod realtime_api;
#[cfg(target_arch = "wasm32")]
mod start_realtime;
pub use realtime_api::*;
#[cfg(target_arch = "wasm32")]
pub use start_realtime::*;
pub mod types;

use beet_core::prelude::*;
use bevy::prelude::*;

pub struct OpenAiKey;

impl OpenAiKey {
	/// Load the `OPENAI_API_KEY` from the environment variables.
	pub fn get() -> Result<String> { env_ext::var("OPENAI_API_KEY")?.xok() }
}
