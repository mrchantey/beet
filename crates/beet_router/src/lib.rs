#![doc = include_str!("../README.md")]
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![allow(async_fn_in_trait)]

#[cfg(feature = "parser")]
pub use beet_router_parser;
pub mod file_router;
pub mod static_file_router;


pub mod prelude {
	pub use crate::file_router::*;
	pub use crate::static_file_router::*;
	pub use crate::DefaultFileRouter;
	#[cfg(feature = "parser")]
	pub use beet_router_parser::prelude::*;
}


pub type DefaultFileRouter =
	static_file_router::StaticFileRouter<static_file_router::DefaultAppState>;



#[cfg(any(test, feature = "_test_site"))]
pub mod test_site {
	pub mod components;
	pub mod routes;
	pub use components::*;
}
