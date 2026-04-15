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
#[cfg(feature = "std")]
mod exit_status;
mod multimap;
mod option;
mod path;
mod result_x;
mod str;
#[cfg(feature = "json")]
mod value;
mod vec;

pub use duration::*;
#[cfg(feature = "std")]
pub use exit_status::*;
pub use multimap::*;
pub use option::*;
pub use path::*;
pub use result_x::*;
pub use str::*;
#[cfg(feature = "json")]
pub use value::*;
pub use vec::*;
