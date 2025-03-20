#![doc = include_str!("../README.md")]
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(async_fn_in_trait)]
#![feature(more_qualified_paths)]

#[cfg(all(feature = "parser", not(target_arch = "wasm32")))]
pub use beet_router_parser;
pub mod app_router;
#[cfg(feature = "bevy")]
pub mod bevy;
pub mod collections;
#[cfg(feature = "serde")]
pub mod file_group;
pub mod file_router;
pub mod pipelines;
#[cfg(all(feature = "parser", not(target_arch = "wasm32")))]
pub mod spa_template;


pub mod prelude {
	pub use crate::app_cx;
	pub use crate::app_router::*;
	#[cfg(feature = "bevy")]
	#[allow(unused_imports)]
	pub use crate::bevy::*;
	pub use crate::collections::*;
	#[cfg(feature = "serde")]
	pub use crate::file_group::*;
	pub use crate::file_router::*;
	pub use crate::pipelines::*;
	#[cfg(all(feature = "parser", not(target_arch = "wasm32")))]
	pub use crate::spa_template::*;
	#[cfg(all(feature = "parser", not(target_arch = "wasm32")))]
	pub use beet_router_parser::prelude::*;

	pub use sweet::prelude::GlobFilter;
}


#[cfg(any(test, feature = "_test_site"))]
pub mod test_site;
