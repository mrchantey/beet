#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

#[cfg(feature = "bevy")]
pub mod bevy;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub mod server;

pub mod prelude {
	#[cfg(feature = "bevy")]
	pub use crate::bevy::*;
	pub use crate::bevybail;
	#[cfg(feature = "bevy")]
	pub use crate::bevyhow;
	#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
	pub use crate::server::*;
}
