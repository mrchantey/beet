//! Turns documents into scenes, and instantiates scenes within one another.
//!
//! - [`bsx`]: the BSX grammar with HTML and XML, its directives and the one
//!   lowering every markup format shares.
//! - `markdown`: markdown, mdx and frontmatter.
//! - `format`: the structured formats, plain text and javascript, and a
//!   document parsed into a world.
//! - [`body`]: a [`SceneBody`](beet_core::prelude::SceneBody) to and from a
//!   world, what loads one, saves one and edits one.
//! - `instance`: an [`Instance`](beet_core::prelude::Instance) names another
//!   document of its package. Activating it loads the target through the parser
//!   its format selects, holds it as data
//!   ([`InstancePrefab`](prelude::InstancePrefab)) and builds that under the use
//!   site, each use with its own entities. See
//!   [`Activation`](prelude::Activation) for the whole of what an activation
//!   does.
//!
//! What names the data and nothing else (the body, its keys, the schemas, the
//! format registry, the template substrate) is `beet_core`'s, so a leaf crate
//! declares a type and a document loads whole without linking a parser.
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

beet_core::test_main!();

pub mod body;
pub mod bsx;
mod format;
mod instance;
mod markdown;
#[cfg(feature = "json")]
mod scene_blob;
mod scene_plugin;

/// Exports the most commonly used items.
pub mod prelude {
	pub use crate::body::*;
	pub use crate::bsx::*;
	pub use crate::format::*;
	pub use crate::instance::*;
	pub use crate::markdown::*;
	#[cfg(feature = "json")]
	pub use crate::scene_blob::*;
	pub use crate::scene_plugin::*;
}
