#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet_test::test_runner))]
#![allow(async_fn_in_trait)]


mod dom_utils;
pub use self::dom_utils::*;
mod logging;
pub use self::logging::*;
mod extensions;
pub use self::extensions::*;
mod net;
pub use self::net::*;


pub mod prelude {
	pub use crate::dom_utils::*;
	pub use crate::extensions::*;
	pub use crate::logging::*;
	pub use crate::net::*;
	pub use wasm_bindgen_futures::spawn_local;
}
