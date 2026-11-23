//! Syntax tree to world: the build walk and the resolver hooks a host
//! installs into it, run after every directive is rewritten away.

mod binding;
mod build_cfg;
mod directives;
mod element;
mod entity_refs;
mod literal;
#[cfg(feature = "bevy_async")]
mod remote;
mod resolve;
mod spread;
mod tag_resolver;
mod uppercase;
mod xml;

pub use build_cfg::*;
pub(in crate::bsx) use directives::*;
pub(in crate::bsx) use literal::literal_to_value;
pub use resolve::*;
pub(in crate::bsx) use spread::*;
pub use tag_resolver::*;
pub use uppercase::*;
