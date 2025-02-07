#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
//!
//! All about rsx trees, html, hydrating patterns, signals.
//! beet_rsx has many features but by default it is quite
//! lightweight and intended to run on constrained devices like the ESP32
//!
//!
pub mod error;
pub mod html;
pub mod hydration;
pub mod rsx;
pub mod signals_rsx;
pub mod string_rsx;
pub mod tree;
#[cfg(feature = "macros")]
pub use beet_rsx_macros::*;
#[cfg(feature = "parser")]
pub use beet_rsx_parser;

#[rustfmt::skip]
pub mod prelude {
	#[cfg(feature = "macros")]
	pub use beet_rsx_macros::*;
	#[cfg(feature = "parser")]
	pub use beet_rsx_parser::prelude::*;
	pub use crate::hydration::*;
	pub use crate::error::*;
	pub use crate::html::*;
	pub use crate::tree::*;
	pub use crate::rsx::*;


	pub type HashMap<K,V> = rapidhash::RapidHashMap<K,V>;
	pub type HashSet<K> = rapidhash::RapidHashSet<K>;

	#[cfg(test)]
	pub use crate::as_beet::*;
}

// rsx macros expect 'beet'
// so import this
// `use beet_rsx::as_beet::*;`
// only for internal examples
#[cfg(debug_assertions)]
pub mod as_beet {
	pub use crate::prelude::*;
	// expose macro for single import in docs
	pub use beet_rsx_macros::rsx;
	pub mod beet {
		pub use crate::*;
		// in beet the beet_rsx crate is aliased to rsx
		pub mod rsx {
			pub use crate::*;
		}
	}
}
