#![doc = include_str!("../README.md")]

#[cfg(test)]
beet::test_main!();

// the dev commands link the native-only fs/infra/webdriver surface, so the whole
// module (and the `CliCommandsPlugin` it defines) is native-only.
#[cfg(not(target_arch = "wasm32"))]
mod commands;

// the winit render-path runtime (window lifecycle + screenshot verification), added
// on top of `BeetPlugins` by the binary when the `winit` feature links the windowed
// render stack. Native-only: the winit/wgpu window has no wasm target here.
#[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
mod render;

// the Cloudflare Worker runner seam: the `WorkersPlugin` (no-op runner + per-isolate
// world cell) + the shared `build_app`, used by the Worker entry.
#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
mod workers;

// the Cloudflare Worker entry: a wasm `#[event(fetch)]` that loads the entry from
// R2 and serves it through the render router. See [`worker_entry`].
#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
mod worker_entry;

// this binary's compiled cargo features, which only it knows: the entry loader
// itself is `beet::launch`, shared with every other beet binary.
pub mod registration;

pub mod prelude {
	#[cfg(not(target_arch = "wasm32"))]
	pub use crate::commands::*;
	pub use crate::registration;
	#[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
	pub use crate::render::*;
	#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
	pub(crate) use crate::workers::*;
}
