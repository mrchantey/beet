//! The perceive-act embodied-agent tools (see `.agents/plans/perceive-act.md`).
//!
//! A floor robot that perceives one photo at a time and acts on what it sees:
//! [`TakePhoto`] (look + describe), [`Remark`] (speak in character), [`Drive`]
//! (choose a [`Heading`]) and [`SetEmotion`] (set a face). [`TakePhoto`] and
//! [`Remark`] are live; [`Drive`] and [`SetEmotion`] are mocked for v1, recording
//! their result in a resource the body/face will read in v2.
mod perceive_act_plugin;
pub use perceive_act_plugin::*;
mod take_photo;
pub use take_photo::*;
mod remark;
pub use remark::*;
mod drive;
pub use drive::*;
mod set_emotion;
pub use set_emotion::*;
pub mod speech_ext;
