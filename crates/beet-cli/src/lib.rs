#![doc = include_str!("../README.md")]

#[cfg(test)]
beet::test_main!();

// the dev commands link the native-only fs/infra/webdriver surface, so the whole
// module (and the `CliCommandsPlugin` it defines) is native-only.
#[cfg(not(target_arch = "wasm32"))]
mod commands;
#[cfg(feature = "thread")]
mod thread_examples;

// the Cloudflare Worker entry: a wasm `#[event(fetch)]` that loads the site from
// R2 and serves it through the render router. See [`worker_entry`].
#[cfg(all(target_arch = "wasm32", feature = "cloudflare"))]
mod worker_entry;

mod serve_plugins;
// the cross-platform site build core (read a store + build the entry into a root),
// shared by the native binary, the wasm Worker, and the check/export-static commands.
mod site_build;

pub mod prelude {
	#[cfg(not(target_arch = "wasm32"))]
	pub use crate::commands::*;
	pub use crate::serve_plugins::*;
	pub use crate::site_build::*;
	#[cfg(feature = "thread")]
	pub use crate::thread_examples::*;
}
