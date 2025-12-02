//! The beet cli is a thin wrapper of this module, so that the cli can be easily forked
//! in the case where custom logic is needed.


mod beet_lock;
pub use beet_lock::*;
