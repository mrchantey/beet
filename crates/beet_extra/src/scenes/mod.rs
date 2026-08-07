//! Scene building blocks, authored as `#[template]` forms so a `.bsx` scene can
//! name them directly.
#[cfg(feature = "ml")]
pub mod ml;
mod templates;
pub use templates::*;
