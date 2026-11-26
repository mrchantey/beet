//! Directives as rewrites: a `bx:` directive is sugar for explicit BSX,
//! rewritten between parsing and the build, touching no world.
mod directive;
mod standard;
pub use directive::*;
pub use standard::*;
