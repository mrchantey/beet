//! Communicate with a GPT-4o class model in real time using WebRTC or WebSockets. Supports text and audio inputs and ouputs, along with audio transcriptions.
//! ## References
//! [Guide](https://platform.openai.com/docs/guides/realtime)
//! [API Reference](https://platform.openai.com/docs/api-reference/realtime)
mod realtime_api;
pub use realtime_api::*;
pub mod types;
