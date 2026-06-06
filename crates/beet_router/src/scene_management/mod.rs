//! Loading, swapping, watching and remotely controlling beet scenes.
//!
//! A *scene* is a reflection-serialized slice of an ECS world; scene management
//! is the machinery to make it the live behaviour of a process:
//! - [`scene_root`]: the shared core — the [`BeetSceneRoot`] marker, the
//!   [`ResetScene`] event and [`set_scene`], which swaps the active scene.
//! - [`scene_server`]: an HTTP API (no_std-friendly) whose real routes arrive as
//!   a POSTed scene; runs equally on a host or on bare-metal firmware.
//! - [`scene_watcher`]: the host CLI side — load, watch and reload a `beet.json`.
//! - [`scene_not_found`]: the welcome page shown when no `beet.json` exists.
//! - [`remote`]: a router-as-CLI that fetches a scene-server device.
//!
//! The core and server need `world_serde`; the watcher, welcome page and remote
//! control are std-only.

// Shared core + HTTP scene server: no_std-friendly, gated on `world_serde` since
// both load/save scenes through world serde.
#[cfg(feature = "world_serde")]
mod scene_root;
#[cfg(feature = "world_serde")]
pub use scene_root::*;
#[cfg(feature = "world_serde")]
mod scene_server;
#[cfg(feature = "world_serde")]
pub use scene_server::*;

// Host CLI side: file watcher (needs std fs + the shared `set_scene`) and the
// welcome page (needs the std beet_ui render pipeline).
#[cfg(all(feature = "std", feature = "world_serde"))]
mod scene_watcher;
#[cfg(all(feature = "std", feature = "world_serde"))]
pub use scene_watcher::*;
#[cfg(feature = "std")]
mod scene_not_found;
#[cfg(feature = "std")]
pub use scene_not_found::*;

// Remote control: a CLI that fetches a device (std http client).
#[cfg(feature = "std")]
mod remote;
#[cfg(feature = "std")]
pub use remote::*;
