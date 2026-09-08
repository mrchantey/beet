//! Runtime verification that the running binary was compiled with what a
//! document needs: crates declare their compiled surface via
//! [`crate_registration!`], and documents ask about it through the one cfg
//! condition grammar, either by excluding a branch (`bx:cfg`) or by refusing
//! to load at all ([`RequireCfg`]).
mod crate_registration;
mod require_cfg;
pub use crate_registration::*;
pub use require_cfg::*;
