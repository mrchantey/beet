mod beet_example_plugins;
pub use self::beet_example_plugins::*;

// the spatial/render example set: only compiled with the bevy render stack.
#[cfg(feature = "bevy_default")]
pub mod beet_example_plugin;
#[cfg(feature = "bevy_default")]
#[allow(unused_imports)]
pub use self::beet_example_plugin::*;

// the agent-thread example tools: headless-friendly, so independent of bevy_default.
#[cfg(feature = "thread")]
mod thread_examples;
#[cfg(feature = "thread")]
pub use self::thread_examples::*;
