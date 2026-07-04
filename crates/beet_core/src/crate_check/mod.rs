//! Runtime verification that the running binary was compiled with required
//! cargo features: crates declare their compiled surface via
//! [`crate_registration!`], entries declare requirements via [`CrateCheck`].
mod crate_check;
mod crate_registration;
pub use crate_check::*;
pub use crate_registration::*;
