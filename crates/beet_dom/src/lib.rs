#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(async_fn_in_trait)]

mod node;
mod utils;

#[cfg(all(feature = "chrome_dev_tools", not(target_arch = "wasm32")))]
mod chrome;
// #[cfg(all(feature = "webdriver", not(target_arch = "wasm32")))]
// mod webdriver;

pub mod prelude {
	#[cfg(all(feature = "chrome_dev_tools", not(target_arch = "wasm32")))]
	pub use crate::chrome::*;
	// #[cfg(all(feature = "webdriver", not(target_arch = "wasm32")))]
	// pub use crate::webdriver::*;

	pub use crate::node::*;
	pub use crate::utils::*;

	#[cfg(feature = "tokens")]
	pub use beet_core::prelude::ToTokens;

	#[cfg(feature = "tokens")]
	pub(crate) use beet_core::exports::*;
}
