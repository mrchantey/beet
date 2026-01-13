//! This module should be in `sweet` but exists here to avoid the issue of cyclic dependencies.
//! The sweet runner depends on internal crates like `beet_net` and those crates use `sweet` for testing.
//! While this is technically allowed by cargo because a crate's test build is seperate from the library build,
//! it should be avoided during development for two major reasons:
//! - Compilation times: A small change to `beet_core` would trigger recompilation of sweet and all its internal deps as well as the crate itself.
//! - Rust Analyzer: Rust Analyzer [gives up](https://github.com/rust-lang/rust-analyzer/issues/14167) when coming across this kind of cyclic dependency.

mod close_to;
mod matcher_control_flow;
mod matcher_not;
mod matcher_vec;
pub mod pretty_diff;
pub use matcher_control_flow::*;
pub use matcher_not::*;
pub use matcher_vec::*;
pub mod panic_ext;
pub use self::close_to::*;
mod matcher_bool;
pub use self::matcher_bool::*;
mod matcher_close;
pub use self::matcher_close::*;
mod matcher_eq;
pub use self::matcher_eq::*;
mod matcher_option;
pub use self::matcher_option::*;
mod matcher_ord;
pub use self::matcher_ord::*;
mod matcher_result;
pub use self::matcher_result::*;
mod matcher_str;
pub use self::matcher_str::*;
mod snapshot;
pub use snapshot::*;
