#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(if_let_guard, result_flattening)]
mod app_router;
#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
mod axum_utils;
#[cfg(all(feature = "chrome", not(target_arch = "wasm32")))]
mod chrome;
mod handlers;
mod http_utils;
#[cfg(all(feature = "lambda", not(target_arch = "wasm32")))]
mod lambda_utils;
mod object_storage;
#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
mod server;
mod templates;
mod transport;

pub mod prelude {
	pub use crate::handlers::*;
	pub use crate::http_utils::*;
	pub use crate::object_storage::*;
	pub use crate::templates::*;

	#[cfg(all(feature = "chrome", not(target_arch = "wasm32")))]
	pub use crate::chrome::*;
	pub use http::StatusCode;
	pub use url::Url;


	pub use crate::app_router::*;
	#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
	pub use crate::axum_utils::*;
	#[cfg(all(feature = "lambda", not(target_arch = "wasm32")))]
	pub use crate::lambda_utils::*;
	#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
	pub use crate::server::*;

	pub(crate) use internal::*;
	#[allow(unused_imports)]
	mod internal {
		pub use beet_core::as_beet::*;
		pub use beet_rsx::prelude::*;
		pub use beet_utils::prelude::*;
	}
	pub use bevy::tasks::futures_lite::StreamExt;
}


pub mod exports {
	pub use bevy::tasks::futures_lite;
	pub use eventsource_stream;
	pub use http;
	pub use http_body_util;
	pub use url;
}
