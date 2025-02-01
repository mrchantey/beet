use bevy::prelude::*;

/// User messages received either internally or externally, can be treated like an StdIn.
#[derive(Debug, Default, Clone, Deref, DerefMut, Event)]
#[cfg_attr(
	feature = "reflect",
	derive(serde::Serialize, serde::Deserialize, Reflect)
)]
pub struct OnUserMessage(pub String);

impl OnUserMessage {
	pub fn new(s: impl Into<String>) -> Self { Self(s.into()) }
}
