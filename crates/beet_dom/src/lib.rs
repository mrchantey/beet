#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(async_fn_in_trait)]

#[cfg(feature = "webdriver")]
mod webdriver;
mod node;
mod utils;

pub mod prelude {

	#[cfg(feature = "webdriver")]
	pub use crate::webdriver::*;
	pub use crate::node::*;
	pub use crate::utils::*;

	#[cfg(feature = "tokens")]
	pub use beet_core::prelude::ToTokens;

	#[cfg(feature = "tokens")]
	pub(crate) use beet_core::exports::*;
}
