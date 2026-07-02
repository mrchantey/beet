//! The browser [`WebHead`]: a wasm socket client serving the perceive-act head.
//!
//! Mirrors the in-process `<MockHead>` (the `thread`-gated `perceive_act` module),
//! but in a browser tab: its own top-level root (route-tree isolation) connecting
//! back to the agent's socket server as the `head` client, serving `take-photo`
//! (webcam), `speak-text` (Web Speech) and `set-emotion` (a rendered `<img>` face).
//! No `BlobStore`: the webcam replaces disk capture.
//!
//! The `WhoAmI`/`ClientRole`/`connect_with_retry` client primitives and the JSON wire
//! types ([`Emotion`], [`SetEmotionInput`], [`SpeakTextInput`]) come from
//! `perceive_act_core`, shared verbatim with the native agent - no local mirror.
use super::*;
use beet_core::prelude::*;
use beet_net::sockets::*;
use beet_router::prelude::*;

/// The browser head: connects to the agent's socket server as the `head` client and
/// serves the head capabilities from the browser (webcam, speech, a rendered face).
///
/// Its own top-level root, so its route tree stays isolated from the agent's
/// identically-named capabilities, exactly like `<MockHead>`. Spawn it from a head
/// program `.bsx` the wasm binary runs (`<WebHead url="ws://.."/>`).
#[template(system)]
pub fn WebHead(
	/// The agent's socket url, eg `ws://127.0.0.1:8338`.
	#[prop(into)]
	url: String,
	mut commands: Commands,
) {
	commands.spawn((
		connect_with_retry(url),
		ExchangeSocket::json(),
		Router,
		ClientRole(SmolStr::new("head")),
		children![WhoAmI, TakePhoto, SpeakText, SetEmotion],
	));
}
