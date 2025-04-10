#![doc = include_str!("../README.md")]
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(async_fn_in_trait)]
#![feature(more_qualified_paths, if_let_guard)]

pub mod app_router;
#[cfg(feature = "bevy")]
pub mod bevy;
#[cfg(feature = "build")]
pub mod build;
pub mod collections;
pub mod file_router;
#[cfg(all(feature = "parser", not(target_arch = "wasm32")))]
pub mod parser;

pub mod prelude {
	pub use crate::app_cx;
	pub use crate::app_router::*;
	#[cfg(feature = "bevy")]
	#[allow(unused_imports)]
	pub use crate::bevy::*;
	#[cfg(feature = "build")]
	pub use crate::build::*;
	pub use crate::collections::*;
	pub use crate::file_router::*;
	#[cfg(all(feature = "parser", not(target_arch = "wasm32")))]
	pub use crate::parser::*;

	// re-exports
	pub use http;
	#[cfg(feature = "parser")]
	pub use ron;
	pub use sweet::prelude::GlobFilter;
	#[cfg(feature = "build")]
	pub use syn;
}


pub mod as_beet {
	pub use crate::prelude::*;
	pub use beet_rsx::as_beet::*;
}

#[cfg(any(test, feature = "_test_site"))]
pub mod test_site;
