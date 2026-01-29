//! Communicate with a GPT-4o class model in real time using WebRTC or WebSockets. Supports text and audio inputs and ouputs, along with audio transcriptions.
//! ## References
//! [Guide](https://platform.openai.com/docs/guides/realtime)
//! [API Reference](https://platform.openai.com/docs/api-reference/realtime)
#[cfg(target_arch = "wasm32")]
mod connect_webrtc;
mod realtime_api;
#[cfg(target_arch = "wasm32")]
pub(self) use connect_webrtc::*;
pub use realtime_api::*;
mod types;
pub use types::*;
