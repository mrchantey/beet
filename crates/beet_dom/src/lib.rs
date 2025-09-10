#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

pub mod node;
pub mod utils;

// #[cfg(all(feature = "chrome_dev_tools", not(target_arch = "wasm32")))]
// mod chrome;


#[cfg(target_arch = "wasm32")]
pub mod web;

pub mod prelude {
	pub use crate::node::*;
	pub use crate::utils::*;
	#[cfg(target_arch = "wasm32")]
	pub use crate::web::prelude::*;

	#[cfg(feature = "tokens")]
	pub use beet_core::prelude::ToTokens;

	#[cfg(feature = "tokens")]
	pub(crate) use beet_core::exports::*;
}


pub mod exports {
	#[cfg(target_arch = "wasm32")]
	pub use js_sys;
	#[cfg(target_arch = "wasm32")]
	pub use wasm_bindgen;
	#[cfg(target_arch = "wasm32")]
	pub use wasm_bindgen_futures;
	#[cfg(target_arch = "wasm32")]
	pub use web_sys;
}
