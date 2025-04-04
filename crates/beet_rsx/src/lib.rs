#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![cfg_attr(feature = "bevy", feature(unboxed_closures, fn_traits))]
#![feature(more_qualified_paths)]
// #![deny(missing_docs)]
//!
//! All about rsx trees, html, hydrating patterns, signals.
//! beet_rsx has many features but by default it is quite
//! lightweight and intended to run on constrained devices like the ESP32
//!
//!
pub mod context;
pub mod dom;
pub mod error;
pub mod html;
pub mod rsx;
pub mod sigfault;
pub mod string_rsx;
#[cfg(feature = "macros")]
pub use beet_rsx_macros::*;
#[cfg(feature = "parser")]
pub use beet_rsx_parser;
#[cfg(feature = "bevy")]
pub mod bevy;
#[cfg(feature = "bevy")]
pub use crate::bevy as bevy2;


#[rustfmt::skip]
pub mod prelude {
	#[cfg(feature = "macros")]
	pub use beet_rsx_macros::*;
	#[cfg(feature = "parser")]
	pub use beet_rsx_parser::prelude::*;
	pub use crate::context::*;
	pub use crate::dom::*;
	pub use crate::error::*;
	pub use crate::html::*;
	pub use crate::rsx::*;
	#[cfg(feature = "bevy")]
	pub use crate::bevy::*;


	pub type HashMap<K,V> = rapidhash::RapidHashMap<K,V>;
	pub type HashSet<K> = rapidhash::RapidHashSet<K>;
	
}

pub mod exports {
	#[cfg(feature = "parser")]
	pub use proc_macro2;
	#[cfg(feature = "parser")]
	pub use quote;
	#[cfg(feature = "serde")]
	pub use ron;
	#[cfg(feature = "serde")]
	pub use serde;
	pub use sweet::prelude::WorkspacePathBuf;
}

// rsx macros expect 'beet'
// so import this
// `use beet_rsx::as_beet::*;`
// only for internal examples
// #[cfg(debug_assertions)]
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
