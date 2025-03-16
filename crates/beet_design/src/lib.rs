#![deny(missing_docs)]
#![doc = include_str!("../README.md")]
/// Structs for use as context in components
pub mod context;
/// Collection of interactive components
pub mod interactive;
/// Collection of layout components
pub mod layout;



/// Commonly used components for beet_design
pub mod prelude {
	pub use crate::context::*;
	pub use crate::interactive::*;
	pub use crate::layout::*;
	pub(crate) use beet_rsx::as_beet::*;
}
