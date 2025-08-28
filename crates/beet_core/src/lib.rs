#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

#[cfg(feature = "http")]
pub mod http_utils;

pub mod actions;
pub mod node;
#[cfg(feature = "tokens")]
pub mod tokens_utils;

pub use beet_core_macros::*;

#[cfg(feature = "bevy")]
mod bevy_utils;
#[cfg(feature = "net")]
pub mod net;
#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
pub mod server;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub mod web;

pub mod prelude {
	pub use crate::actions::*;
	#[cfg(feature = "bevy")]
	pub use crate::bevy_utils::*;
	pub use crate::bevybail;
	#[cfg(feature = "bevy")]
	pub use crate::bevyhow;
	#[cfg(feature = "http")]
	pub use crate::http_utils::*;
	pub use crate::node::*;
	pub use crate::pkg_config;
	#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
	pub use crate::server::*;
	#[cfg(feature = "tokens")]
	pub use crate::tokens_utils::*;
	#[cfg(all(feature = "web", target_arch = "wasm32"))]
	pub use crate::web::prelude::*;
	pub use beet_core_macros::*;
}


pub mod as_beet {
	pub use crate::prelude::*;
	pub use crate::*;
	pub mod beet {
		pub use crate::*;
		pub mod prelude {
			pub use crate::prelude::*;
		}
	}
}


pub mod exports {
	#[cfg(feature = "http")]
	pub use http;
	#[cfg(feature = "http")]
	pub use http_body_util;
	#[cfg(feature = "tokens")]
	pub use proc_macro2;
	#[cfg(feature = "tokens")]
	pub use quote;
	// #[cfg(all(feature = "net", not(target_arch = "wasm32")))]
	// pub use reqwest;
	#[cfg(feature = "serde")]
	pub use ron;
	pub use send_wrapper::SendWrapper;
	#[cfg(feature = "tokens")]
	pub use syn;
	#[cfg(feature = "serde")]
	pub use toml;
	#[cfg(feature = "http")]
	pub use url;
	#[cfg(feature = "http")]
	pub use url::Url;
}
