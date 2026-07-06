//! The wasm browser **head** for the perceive-act demo (v3).
//!
//! A `web`-feature wasm binary serves this to a browser tab, where it connects back to
//! the perceive-act agent's socket server as the `head` client (mirroring the desktop
//! `<MockHead>`) and serves the head capabilities from the browser: [`TakePhoto`] from
//! the real webcam, [`SpeakText`] via the Web Speech API, and [`ShowImage`] as a
//! rendered `<img>` face. The agent forwards each capability over the socket, so the
//! In/Out wire types match its `perceive_act` definitions.
//!
//! This module is deliberately independent of the `thread`-gated `perceive_act` module
//! (which pulls the native LLM `beet_thread`, absent from a wasm build): it needs only
//! the `web` base (`Socket`/`ExchangeSocket`/`Router`/`#[action]`) plus the browser
//! web-sys APIs. The shared wire types + client primitives come from `perceive_act_core`.
// the wire types (`DisplayedImage`, the tool inputs) + client primitives (`WhoAmI`,
// `ClientRole`, `PersistentSocket`), mirrored once in `perceive_act_core` and shared
// with the native agent, so the head does not redefine them.
pub use crate::perceive_act_core::*;
mod perceive_act_web_plugin;
pub use perceive_act_web_plugin::*;
mod web_head;
pub use web_head::*;
mod take_photo;
pub use take_photo::*;
mod speak_text;
pub use speak_text::*;
mod render_face;
pub use render_face::*;
