//! Syntax tree to world: the build walk, the event seam, and the resolver
//! hooks a host installs into it.

mod binding;
mod build_cfg;
mod directives;
mod element;
mod entity_refs;
mod events;
mod literal;
#[cfg(feature = "bevy_async")]
mod remote;
mod resolve;
mod spread;
mod style_resolver;
mod tag_resolver;
mod uppercase;

pub use build_cfg::*;
pub(in crate::bsx) use directives::*;
pub use events::*;
pub use resolve::*;
pub(in crate::bsx) use spread::*;
pub use style_resolver::*;
pub use tag_resolver::*;
pub use uppercase::*;
