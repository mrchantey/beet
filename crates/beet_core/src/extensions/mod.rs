//! Extension traits for standard library and common types.
//!
//! This module provides additional methods on existing types through extension
//! traits, enabling more ergonomic APIs and method chaining.
//!
//! # Traits
//!
//! - [`DurationExt`] - Additional methods for [`Duration`](std::time::Duration)
//! - [`ExitStatusExt`] - Convert exit status to [`Result`]
//! - [`VecExt`] - Additional vector operations
//! - [`OptionExt`] - Additional option operations
//! - [`ResultXExt`] - Additional result operations
//! - [`Multimap`] - Multi-value map operations

mod duration;
mod exit_status;
mod multimap;
mod option;
mod result_x;
#[cfg(feature = "json")]
mod value;
mod vec;

pub use duration::*;
pub use exit_status::*;
pub use multimap::*;
pub use option::*;
pub use result_x::*;
#[cfg(feature = "json")]
pub use value::*;
pub use vec::*;
