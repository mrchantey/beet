#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![cfg_attr(feature = "nightly", feature(fn_traits, unboxed_closures))]
#![feature(let_chains)]
// #![deny(missing_docs)]
//!
pub use beet_rsx_macros::*;
pub mod reactivity;
pub mod templating;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[rustfmt::skip]
pub mod prelude {
	pub use beet_rsx_macros::*;
	pub use crate::reactivity::*;
	pub use crate::templating::*;
	#[cfg(target_arch = "wasm32")]
	pub use crate::wasm::*;
}

pub mod exports {
	pub use anyhow;
}

// rsx macros expect 'beet'
// so import this
// `use beet_rsx::as_beet::*;`
// #[cfg(debug_assertions)]
/// Internal use only
pub mod as_beet {
	pub use beet::prelude::*;
	pub mod beet {
		pub mod prelude {
			pub use crate::prelude::*;
			pub use beet_core::prelude::*;
			pub use beet_utils::prelude::*;
			#[allow(unused)]
			pub(crate) use bevy::prelude::*;
		}
		pub mod exports {
			pub use crate::exports::*;
		}
	}
}
