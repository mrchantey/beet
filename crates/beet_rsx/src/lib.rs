#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

pub mod error;
pub mod html;
pub mod hydration;
pub mod rsx;
pub mod signals_rsx;
pub mod string_rsx;
pub mod tree;
pub use beet_rsx_macros::rsx;

#[rustfmt::skip]
pub mod prelude {
	pub use beet_rsx_macros::rsx;
	pub use crate::hydration::*;
	pub use crate::error::*;
	pub use crate::html::*;
	pub use crate::tree::*;
	pub use crate::rsx::*;

	// rsx macro expects `beet`
	#[cfg(test)]
	pub use crate as beet;
}
