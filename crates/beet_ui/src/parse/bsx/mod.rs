//! The default BSX event vocabulary: [`BsxDefaultsPlugin`], and the `click`
//! handler it adds in `event_script`. The parser itself is `beet_core`'s
//! [`BsxParser`](beet_core::prelude::BsxParser), a base format.
mod defaults;
pub use defaults::*;
#[cfg(feature = "scripting")]
mod event_script;
