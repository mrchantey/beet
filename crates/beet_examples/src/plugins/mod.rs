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

// the no-code document CRUD route templates: need the router/net deps the `thread`
// feature pulls, headless-friendly.
#[cfg(feature = "thread")]
mod doc_crud;
#[cfg(feature = "thread")]
pub use self::doc_crud::*;

// the headless behaviour-tree example: needs the net deps the `thread` feature
// pulls (the boot verb), headless-friendly.
#[cfg(feature = "thread")]
mod behavior_examples;
#[cfg(feature = "thread")]
pub use self::behavior_examples::*;

// the agent calculator toolset + the `Behavior` sequence marker.
#[cfg(feature = "thread")]
mod agent_examples;
#[cfg(feature = "thread")]
pub use self::agent_examples::*;
