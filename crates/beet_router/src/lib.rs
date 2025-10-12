#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(if_let_guard)]
mod actions;
mod app_router;
#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
mod axum_utils;
mod flow_router;
mod handlers;
#[cfg(all(feature = "lambda", not(target_arch = "wasm32")))]
mod lambda_utils;
mod templates;

pub mod prelude {
	pub use crate::actions::*;
	pub use crate::flow_router::*;
	pub use crate::handlers::*;
	pub use crate::templates::*;

	pub use crate::app_router::*;
	#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
	pub use crate::axum_utils::*;
	#[cfg(all(feature = "lambda", not(target_arch = "wasm32")))]
	pub use crate::lambda_utils::*;
}


pub mod exports {}
