//! The perceive-act embodied-agent tools (see `.agents/plans/perceive-act.md`).
//!
//! A floor robot that perceives one photo at a time and acts on what it sees:
//! [`TakePhoto`] (look + describe), [`Remark`] (speak in character), [`SetHeading`]
//! (choose a [`Heading`]) and [`SetEmotion`] (set an [`Emotion`]). [`TakePhoto`] and
//! [`Remark`] are live; [`SetHeading`] and [`SetEmotion`] only record their component
//! in v1, which the body/face will read via `Single<&Heading>` / `Single<&Emotion>`
//! in v2.
mod perceive_act_plugin;
pub use perceive_act_plugin::*;
mod take_photo;
pub use take_photo::*;
mod remark;
pub use remark::*;
mod heading;
pub use heading::*;
mod set_emotion;
pub use set_emotion::*;
pub mod speech_ext;
