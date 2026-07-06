//! The perceive-act embodied-agent tools (see `.agents/plans/percieve-act.md`).
//!
//! A floor robot that perceives one photo at a time and acts on what it sees.
//! Each cycle [`PostPhoto`] (the camera actor) captures via [`TakePhoto`] and
//! posts the photo into the thread, then the agent's single model call answers
//! with one [`RespondMultiModalAction`] tool call, which fans out to [`SetEmotion`],
//! [`SpeakText`] and [`RecordDrive`] (or a real body's `drive`) concurrently. The agent forwards each
//! capability over a socket to the client that serves it, bound by the
//! [`capability_socket`] handshake; run standalone, the tools' own local
//! handlers apply.
// the wire types + socket-client primitives shared with the wasm head; re-exported so
// `crate::perceive_act::{Emotion, WhoAmI, ClientRole, ..}` still resolve here.
pub use crate::perceive_act_core::*;
mod perceive_act_plugin;
pub use perceive_act_plugin::*;
mod capability_socket;
pub use capability_socket::*;
mod mock_clients;
pub use mock_clients::*;
// the wgpu render body (v2) swaps the logging mock body for a driven 3d fox; it needs
// the render stack (`CharacterDrive`, `<Foxie>`), so it rides the `bevy_default` set.
#[cfg(feature = "bevy_default")]
mod wgpu_body;
#[cfg(feature = "bevy_default")]
pub use wgpu_body::*;
mod take_photo;
pub use take_photo::*;
mod post_photo;
pub use post_photo::*;
mod respond_multi_modal;
pub use respond_multi_modal::*;
mod speak_text;
pub use speak_text::*;
mod record_drive;
pub use record_drive::*;
mod set_emotion;
pub use set_emotion::*;
pub mod speech_ext;
