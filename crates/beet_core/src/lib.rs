#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

#[cfg(feature = "bevy")]
pub mod bevy;
#[cfg(feature = "net")]
pub mod net;
#[cfg(feature = "net")]
pub use net::cross_fetch;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub mod server;
#[cfg(feature = "web")]
pub mod web;

pub mod prelude {
	#[cfg(feature = "bevy")]
	pub use crate::bevy::*;
	pub use crate::bevybail;
	#[cfg(feature = "bevy")]
	pub use crate::bevyhow;
	#[cfg(feature = "net")]
	pub use crate::net::*;
	#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
	pub use crate::server::*;
	#[cfg(feature = "web")]
	pub use crate::web::prelude::*;
}


pub mod as_beet {
	pub mod beet {
		pub use crate::*;
		pub mod prelude {
			pub use crate::prelude::*;
		}
	}
}


pub mod exports {
	#[cfg(feature = "net")]
	pub use http;
	#[cfg(feature = "net")]
	pub use http_body_util;
	#[cfg(feature = "tokens")]
	pub use proc_macro2;
	#[cfg(feature = "tokens")]
	pub use quote;
	#[cfg(all(feature = "net", not(target_arch = "wasm32")))]
	pub use reqwest;
	#[cfg(feature = "tokens")]
	pub use syn;
}
