#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(more_qualified_paths, let_chains)]
// #![deny(missing_docs)]
//!
//! All about rsx trees, html, hydrating patterns, signals.
//! beet_rsx has many features but by default it is quite
//! lightweight and intended to run on constrained devices like the ESP32
//!
//!
pub use beet_rsx_macros::*;
pub mod html;

#[rustfmt::skip]
pub mod prelude {
	pub use beet_rsx_macros::*;
	pub use crate::html::*;
}

pub mod exports {
	pub use anyhow;
}

// rsx macros expect 'beet'
// so import this
// `use beet_rsx::as_beet::*;`
// only for internal examples
// #[cfg(debug_assertions)]
pub mod as_beet {
	pub use beet::prelude::*;
	pub mod beet {
		pub use crate as rsx;
		pub mod prelude {
			pub use crate::prelude::*;
			pub use beet_common::prelude::*;
			pub use sweet::prelude::*;
		}
		pub mod exports {
			pub use crate::exports::*;
		}
	}
}
