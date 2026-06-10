//! Loading, swapping, watching and remotely controlling beet scenes.
//!
//! A *scene* is a reflection-serialized slice of an ECS world; scene management
//! is the machinery to make it the live behaviour of a process:
//! - [`scene_root`]: the shared core — the [`BeetSceneRoot`] marker, the
//!   [`ResetScene`] event and [`set_scene`], which swaps the active scene.
//! - [`scene_server`]: an HTTP API (no_std-friendly) whose real routes arrive as
//!   a POSTed scene; runs equally on a host or on bare-metal firmware.
//! - [`scene_watcher`]: the host CLI side, wired through ECS — the
//!   [`SceneManagementPlugin`] reactively persists the `.beet` retained scene
//!   cache (observers on [`BeetSceneRoot`]) and a [`SceneWatch`] `on_add` hook
//!   installs the `--watch` file watcher.
//! - [`scene_commands`]: the host load/clear/reset/dump/run commands, each
//!   targeting either a remote device or the local world, plus [`ExportScene`].
//!
//! The core and server need `template_serde`; the watcher, welcome page and host
//! commands are std-only.

// Shared core + HTTP scene server: no_std-friendly, gated on `template_serde` since
// both load/save scenes through world serde.
#[cfg(feature = "template_serde")]
mod scene_root;
#[cfg(feature = "template_serde")]
pub use scene_root::*;
#[cfg(feature = "template_serde")]
mod scene_server;
#[cfg(feature = "template_serde")]
pub use scene_server::*;

// Host CLI side: the file watcher needs native fs (`FsWatcher`/`DirEvent`, absent
// on wasm), so it and the scene commands that spawn it are gated off wasm.
#[cfg(all(feature = "std", feature = "template_serde", not(target_arch = "wasm32")))]
mod scene_watcher;
#[cfg(all(feature = "std", feature = "template_serde", not(target_arch = "wasm32")))]
pub use scene_watcher::*;

// Host scene commands: load/clear/reset/dump/run + export, each local or remote
// (std http client + world serde for the local path). Spawns the file watcher,
// so they share its native-only gate.
#[cfg(all(feature = "std", feature = "template_serde", not(target_arch = "wasm32")))]
mod scene_commands;
#[cfg(all(feature = "std", feature = "template_serde", not(target_arch = "wasm32")))]
pub use scene_commands::*;
