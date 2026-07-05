//! Loading, swapping and remotely pushing beet scenes.
//!
//! A *scene* is a reflection-serialized slice of an ECS world; scene management
//! is the machinery to make it the live behaviour of a process:
//! - [`scene_root`]: the shared core — the [`BeetSceneRoot`] marker, the
//!   [`ResetScene`] event and [`set_scene`], which swaps the active scene.
//! - [`scene_server`]: an HTTP API (no_std-friendly) whose real routes arrive as
//!   a POSTed scene; runs equally on a host or on bare-metal firmware.
//! - [`scene_commands`]: the host push commands (load/clear/reset/dump/run), each
//!   driving a remote scene-server device over HTTP.
//!
//! The core and server need `template_serde`; the host push commands are std-only
//! (an http client). The binary forces none of this on; a device or push host
//! opts in by adding [`SceneManagementPlugin`] and/or spawning a scene server.

// Shared core + HTTP scene server: no_std-friendly, gated on `template_serde`
// since both load/save scenes through world serde.
#[cfg(feature = "template_serde")]
mod scene_root;
#[cfg(feature = "template_serde")]
pub use scene_root::*;
#[cfg(feature = "template_serde")]
mod scene_server;
#[cfg(feature = "template_serde")]
pub use scene_server::*;

// Socket-aware reset: a client socket whose closure triggers [`ResetScene`].
// Rides the socket core (`sockets`) + the reset event's core (`template_serde`);
// no_std-capable, so a bare-metal body halts its hardware when its agent drops.
#[cfg(all(feature = "sockets", feature = "template_serde"))]
mod reset_on_disconnect;
#[cfg(all(feature = "sockets", feature = "template_serde"))]
pub use reset_on_disconnect::*;

// Host push commands: drive a remote device (std http client). Native-only.
#[cfg(all(
	feature = "std",
	feature = "template_serde",
	not(target_arch = "wasm32")
))]
mod scene_commands;
#[cfg(all(
	feature = "std",
	feature = "template_serde",
	not(target_arch = "wasm32")
))]
pub use scene_commands::*;

#[cfg(all(
	feature = "std",
	feature = "template_serde",
	not(target_arch = "wasm32")
))]
use beet_core::prelude::*;

/// Registers the scene-management capabilities as reflect types so a scene (or a
/// `main.bsx`) can name them: the [`BeetSceneRoot`] marker and the host push
/// commands ([`SceneCommandsPlugin`]). The receiving counterpart, the
/// [`scene_server`] HTTP API, is wired by whoever spawns the server.
///
/// Inert by default — the binary forces nothing; a device or push host adds this.
#[cfg(all(
	feature = "std",
	feature = "template_serde",
	not(target_arch = "wasm32")
))]
pub struct SceneManagementPlugin;

#[cfg(all(
	feature = "std",
	feature = "template_serde",
	not(target_arch = "wasm32")
))]
impl Plugin for SceneManagementPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<BeetSceneRoot>()
			.add_plugins(SceneCommandsPlugin);
		#[cfg(feature = "sockets")]
		app.register_type::<ResetOnDisconnect>();
	}
}
