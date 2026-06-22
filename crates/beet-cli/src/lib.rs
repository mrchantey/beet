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

pub mod prelude {
	#[cfg(not(target_arch = "wasm32"))]
	pub use crate::commands::*;
	pub use crate::serve_plugins::*;
	#[cfg(feature = "thread")]
	pub use crate::thread_examples::*;
}
