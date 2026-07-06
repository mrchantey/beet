//! The perceive-act tool wire types shared by the agent and every head client.
//!
//! Pure reflect + serde types, so the JSON the agent forwards over the socket
//! deserializes identically whether the head that serves it is the native mock, the
//! wgpu body, or the wasm browser head. No `beet_thread`, so they build in the wasm
//! `web` head as readily as the native agent.
use beet_core::prelude::*;

/// What to say out loud, the input to the `speak-text` capability.
#[derive(Reflect, serde::Deserialize, serde::Serialize)]
pub struct SpeakTextInput {
	/// The line to say out loud, in character. Keep it short and full of personality.
	pub text: String,
}

/// What image to display, the input to the `show-image` capability.
///
/// `src` is a resolved url the head points its `<img>` at. The agent maps the
/// model's chosen image title (one of the current scene's options) to this url
/// before forwarding, so the head stays a dumb display and never needs to know
/// which scene is active.
#[derive(Reflect, serde::Deserialize, serde::Serialize)]
pub struct ShowImageInput {
	/// The url of the image to display, eg
	/// `/assets/extra/perceive-act/explorer/images/joy.png`.
	pub src: String,
}

/// The image currently shown on the face, recorded by the `show-image` capability
/// and read with `Single<&DisplayedImage>`. Holds the resolved url; the web head
/// renders it, the native mock only records it.
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct DisplayedImage(pub SmolStr);
