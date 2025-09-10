#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![cfg_attr(feature = "nightly", feature(unboxed_closures, fn_traits))]
// #![deny(missing_docs)]
//!
pub use beet_rsx_macros::*;
pub mod apply_snippets;
pub mod reactivity;
pub mod templating;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[rustfmt::skip]
pub mod prelude {
	pub use beet_rsx_macros::*;
	pub use crate::apply_snippets::*;
	pub use crate::reactivity::*;
	pub use crate::templating::*;
	#[cfg(target_arch = "wasm32")]
	pub use crate::wasm::*;
}

pub mod exports {
	pub use anyhow;
}
