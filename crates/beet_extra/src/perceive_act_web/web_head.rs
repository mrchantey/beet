//! The browser [`WebHead`]: a wasm socket client serving the perceive-act head.
//!
//! Mirrors the in-process `<MockHead>` (the `thread`-gated `perceive_act` module),
//! but in a browser tab: its own top-level root (route-tree isolation) connecting
//! back to the agent's socket server as the `head` client, serving `take-photo`
//! (webcam), `speak-text` (Web Speech) and `set-emotion` (a rendered `<img>` face).
//! No `BlobStore`: the webcam replaces disk capture.
//!
//! The `WhoAmI`/`ClientRole` client primitives and the JSON wire types ([`Emotion`],
//! [`SetEmotionInput`], [`SpeakTextInput`]) come from `perceive_act_core`, shared
//! verbatim with the native agent - no local mirror.
use super::*;
use beet_core::prelude::*;
use beet_net::sockets::*;
use beet_router::prelude::*;

/// The browser head: connects to the agent's socket server as the `head` client and
/// serves the head capabilities from the browser (webcam, speech, a rendered face).
///
/// Its own top-level root, so its route tree stays isolated from the agent's
/// identically-named capabilities, exactly like `<MockHead>`. Spawn it from a head
/// program `.bsx` the wasm binary runs (`<WebHead/>`). With no `url` the agent is
/// assumed to be the host serving this page (`ws://<page-host>:8338`), so the same
/// head program works from the serving machine and from a phone on the LAN.
#[template(system)]
pub fn WebHead(
	/// The agent's socket url, eg `ws://192.168.1.7:8338`. Defaults to the page's
	/// own host on the default socket port.
	#[prop(into, default)]
	url: Option<String>,
	mut commands: Commands,
) {
	let url = url.unwrap_or_else(default_agent_url);
	info!("head connecting to agent: {url}");
	commands.spawn((
		PersistentSocket::new(url),
		ExchangeSocket::json(),
		Router,
		ClientRole(SmolStr::new("head")),
		children![WhoAmI, TakePhoto, SpeakText, SetEmotion],
	));
}

/// The agent socket url assumed when none is given: the host serving this page,
/// on the default socket port (the v3 scene runs the head http server and the
/// agent socket server on one machine).
fn default_agent_url() -> String {
	let hostname = web_sys::window()
		.and_then(|window| window.location().hostname().ok())
		.filter(|hostname| !hostname.is_empty())
		.unwrap_or_else(|| "127.0.0.1".to_string());
	format!("ws://{hostname}:8338")
}
