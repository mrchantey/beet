#[cfg(feature = "bevy")]
pub use sweet_bevy as bevy;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub use sweet_fs as fs;
#[cfg(feature = "net")]
pub use sweet_net as net;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub use sweet_server as server;
#[cfg(feature = "test")]
pub use sweet_test as test;
#[cfg(feature = "test")]
pub use sweet_test::sweet_test_macros::*;
#[cfg(feature = "test")]
pub use sweet_test::test_runner;
pub use sweet_utils as utils;
pub use sweet_utils::elog;
pub use sweet_utils::log;
pub use sweet_utils::noop;
#[cfg(feature = "web")]
pub use sweet_web as web;

pub mod prelude {
	#[cfg(feature = "bevy")]
	pub use crate::bevy::prelude::*;
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub use crate::fs::prelude::*;
	#[cfg(feature = "net")]
	pub use crate::net::prelude::*;
	#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
	pub use crate::server::prelude::*;
	#[cfg(feature = "test")]
	pub use crate::test::prelude::*;
	pub use crate::utils::prelude::*;
	#[cfg(feature = "web")]
	pub use crate::web::prelude::*;
}

pub mod exports {
	pub use sweet_utils::exports::*;
}
