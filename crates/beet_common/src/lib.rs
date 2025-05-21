//! low dependency common types and helpers for beet crates.
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(let_chains)]

pub mod bevy_utils;
pub mod node;
pub mod templating;
#[cfg(feature = "tokens")]
pub mod tokens_utils;

pub use beet_common_macros::*;

mod define_tokens_collector;

pub mod prelude {
	pub use crate::bevy_utils::*;
	pub use crate::define_token_collector;
	pub use crate::node::*;
	pub use crate::templating::*;
	#[cfg(feature = "tokens")]
	pub use crate::tokens_utils::*;
	pub use beet_common_macros::*;
}

pub mod exports {

	#[cfg(feature = "tokens")]
	pub use proc_macro2;
	#[cfg(feature = "tokens")]
	pub use quote;
}

pub mod as_beet {
	pub use beet::prelude::*;
	pub mod beet {
		pub use crate as rsx;
		pub mod prelude {
			pub use crate::prelude::*;
			// pub use beet_common::prelude::*;
		}
		pub mod exports {
			pub use crate::exports::*;
		}
	}
}
