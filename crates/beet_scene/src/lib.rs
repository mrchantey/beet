//! Turns documents into scenes, and instantiates scenes within one another.
//!
//! - [`formats`]: the format registry and every format that builds a plain
//!   scene, BSX, HTML, XML, markdown, mdx, the structured formats, plain text
//!   and javascript.
//! - `literal`: an authored literal to the value a scene body holds, shared by
//!   every format.
//! - [`body`]: a [`SceneBody`](beet_core::prelude::SceneBody) to and from a
//!   world, what parses a document onto an entity, loads one, saves one and
//!   edits one.
//! - `instance`: an [`Instance`](beet_core::prelude::Instance) names another
//!   document of its package. Activating it loads the target through the parser
//!   its format selects, holds it as data
//!   ([`InstancePrefab`](prelude::InstancePrefab)) and builds that under the use
//!   site, each use with its own entities. See
//!   [`Activation`](prelude::Activation) for the whole of what an activation
//!   does.
//!
//! What names the data and nothing else (the body, its keys, the schemas, the
//! template substrate) is `beet_core`'s, so a leaf crate
//! declares a type and a document loads whole without linking a parser.
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

beet_core::test_main!();

pub mod body;
pub mod formats;
mod instance;
mod literal;
#[cfg(feature = "json")]
mod scene_blob;
mod scene_plugin;

/// Exports the most commonly used items.
pub mod prelude {
	pub use crate::body::*;
	pub use crate::formats::*;
	pub use crate::instance::*;
	pub use crate::literal::*;
	#[cfg(feature = "json")]
	pub use crate::scene_blob::*;
	pub use crate::scene_plugin::*;
}
