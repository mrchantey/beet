//! In-process mock head and body clients for the perceive-act demo.
//!
//! Each is a socket client on its own root that connects to the agent, announces its
//! role via `whoami`, and serves that role's capabilities with mock effects. Spawned
//! same-process for v1; v2 swaps the body for a real wgpu fox, v3 the head for a
//! browser and the body for a device. The client is its own root so its route tree
//! stays isolated from the agent's identically-named capabilities.
use super::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_net::sockets::*;
use beet_router::prelude::*;

/// The in-process mock head: connects to the agent and serves the head capabilities
/// with mock effects. `take-photo` reads the floor-photo fixtures, `speak-text` speaks
/// via `tts`, and `set-emotion` logs the expression (a rendered face lands with the web
/// head in v3).
#[template(system)]
pub fn MockHead(
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
		// the head owns its capture IO: a disk store rooted at the workspace (the same
		// rooting the CLI uses), so `take-photo` reads the floor-photo fixtures through
		// its own `AncestorQuery<&BlobStore>`. Becomes the webcam in v3.
		BlobStore::new(FsStore::new(
			AbsPathBuf::new(fs_ext::workspace_root())
				.expect("workspace root resolves"),
		)),
		children![WhoAmI, TakePhoto, SpeakText, SetEmotion],
	));
}

/// The in-process mock body: connects to the agent and serves `apply-heading`, logging
/// the heading (a real fox drive lands with the wgpu body in v2).
#[template(system)]
pub fn MockBody(
	/// The agent's socket url, eg `ws://127.0.0.1:8338`.
	#[prop(into)]
	url: String,
	mut commands: Commands,
) {
	commands.spawn((
		connect_with_retry(url),
		ExchangeSocket::json(),
		Router,
		ClientRole(SmolStr::new("body")),
		children![WhoAmI, ApplyHeading],
	));
}

/// Connect to `url`, retrying until the server's listener is up (it binds after the
/// scene builds, so an in-process client races it), then insert the [`Socket`].
pub fn connect_with_retry(url: impl Into<String>) -> OnSpawn {
	let url = url.into();
	OnSpawn::new_async_local(async move |entity| -> Result {
		for attempt in 0..50u32 {
			match Socket::connect(&url).await {
				Ok(socket) => {
					entity.insert(socket).await?;
					return Ok(());
				}
				// keep retrying until the last attempt, then surface the error.
				Err(err) if attempt == 49 => return Err(err),
				Err(_) => time_ext::sleep_millis(100).await,
			}
		}
		Ok(())
	})
}
