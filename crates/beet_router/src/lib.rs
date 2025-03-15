#![doc = include_str!("../README.md")]
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(async_fn_in_trait)]
#![feature(more_qualified_paths)]

#[cfg(all(feature = "parser", not(target_arch = "wasm32")))]
pub use beet_router_parser;
#[cfg(feature = "bevy")]
pub mod bevy;
pub mod file_router;
pub mod spa_template;


pub mod prelude {
	#[cfg(feature = "bevy")]
	#[allow(unused_imports)]
	pub use crate::bevy::*;
	pub use crate::file_router::*;
	pub use crate::spa_template::*;
	#[cfg(all(feature = "parser", not(target_arch = "wasm32")))]
	pub use beet_router_parser::prelude::*;
}


#[cfg(any(test, feature = "_test_site"))]
pub mod test_site {
	pub mod components;
	pub mod routes;
	pub use components::*;
}
