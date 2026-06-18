#![no_std]
#![doc = include_str!("../README.md")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod attribute_group;
mod attribute_map;
mod named_field;
/// Package configuration extensions for Cargo.toml parsing.
#[cfg(feature = "std")]
pub mod pkg_ext;
mod synhow;
mod tokenize;

pub mod prelude {
	pub use crate::attribute_group::*;
	pub use crate::attribute_map::*;
	pub use crate::named_field::*;
	#[cfg(feature = "std")]
	pub use crate::pkg_ext;
	pub use crate::synbail;
	pub use crate::synhow;
	pub use crate::tokenize::*;
}
