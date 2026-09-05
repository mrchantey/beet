//! Making a binary a beet runtime: find an entry document, build it, and let the
//! loaded tree run itself.
//!
//! Split from the capabilities a binary links, which is the other half and is
//! the binary's own business. [`entry_build`] is the target-agnostic core
//! (resolve a store, read its sources, build them into a root); [`LaunchPlugin`]
//! is the `Startup` system that drives it for a running app. A binary composes
//! this with its chosen capability plugins.
pub mod entry_build;
pub use entry_build::ResolvedEntry;
mod launch_plugin;
pub use launch_plugin::*;
