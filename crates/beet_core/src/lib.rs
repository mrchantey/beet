#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(exit_status_error)]

pub mod actions;
#[cfg(feature = "tokens")]
pub mod tokens_utils;

#[cfg(target_arch = "wasm32")]
pub mod web_utils;

pub use beet_core_macros::*;

mod bevy_utils;
mod workspace_config;


pub mod prelude {
	/// macro helper
	#[cfg(not(doctest))]
	#[allow(unused)]
	pub(crate) use crate as beet_core;
	pub use crate::actions::*;
	pub use crate::bevy_utils::*;
	pub use crate::bevybail;
	pub use crate::bevyhow;
	pub use crate::pkg_config;
	#[cfg(feature = "tokens")]
	pub use crate::tokens_utils::*;
	#[cfg(target_arch = "wasm32")]
	pub use crate::web_utils::*;
	pub use crate::workspace_config::*;
	pub use beet_core_macros::*;
}


pub mod exports {
	pub use async_channel;
	pub use futures_lite;
	#[cfg(feature = "tokens")]
	pub use proc_macro2;
	#[cfg(feature = "tokens")]
	pub use quote;
	#[cfg(feature = "serde")]
	pub use ron;
	pub use send_wrapper::SendWrapper;
	#[cfg(feature = "tokens")]
	pub use syn;
	#[cfg(feature = "serde")]
	pub use toml;
}
